// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use futures::future::try_join_all;
use ssh2::{Channel, Session};
use tokio::{net::TcpStream, time::sleep};

use crate::{
    client::Instance,
    ensure,
    error::{SshError, SshResult},
};

#[derive(PartialEq, Eq)]
/// The status of a ssh command running in the background.
pub enum CommandStatus {
    Running,
    Terminated,
}

/// The command to execute on all specified remote machines.
#[derive(Clone)]
pub struct SshCommand<C: Fn(usize) -> String> {
    /// The shell command to execute, parametrized by instance index.
    pub command: C,
    /// Whether to run the command in the background (and return immediately). Commands
    /// running in the background are identified by a unique id.
    pub background: Option<String>,
    /// The path from where to execute the command.
    pub path: Option<PathBuf>,
    /// The log file to redirect all stdout and stderr.
    pub log_file: Option<PathBuf>,
}

impl<C: Fn(usize) -> String> SshCommand<C> {
    /// Create a new ssh command.
    pub fn new(command: C) -> Self {
        Self {
            command,
            background: None,
            path: None,
            log_file: None,
        }
    }

    /// Set id of the command and indicate that it should run in the background.
    pub fn run_background(mut self, id: String) -> Self {
        self.background = Some(id);
        self
    }

    /// Set the path from where to execute the command.
    pub fn with_execute_from_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the log file where to redirect stdout and stderr.
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
        self
    }

    /// Convert the command into a string.
    pub fn stringify(&self, index: usize) -> String {
        let mut str = (self.command)(index);
        if let Some(log_file) = &self.log_file {
            str = format!("{str} |& tee {}", log_file.as_path().display());
        }
        if let Some(id) = &self.background {
            str = format!("tmux new -d -s \"{id}\" \"{str}\"");
        }
        if let Some(exec_path) = &self.path {
            str = format!("(cd {} && {str})", exec_path.as_path().display());
        }
        str
    }

    /// Return whether a background command is still running. Returns `Terminated` if the
    /// command is not running in the background.
    pub fn status(&self, context: &str) -> CommandStatus {
        match &self.background {
            Some(id) if context.contains(id) => CommandStatus::Running,
            _ => CommandStatus::Terminated,
        }
    }
}

#[derive(Clone)]
pub struct SshConnectionManager {
    /// The ssh username.
    username: String,
    /// The ssh primate key to connect to the instances.
    private_key_file: PathBuf,
    /// The timeout value of the connection.
    timeout: Option<Duration>,
    /// The number of retries before giving up to execute the command.
    retries: usize,
}

impl SshConnectionManager {
    /// Delay before re-attempting an ssh execution.
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    /// Create a new ssh manager from the instances username and private keys.
    pub fn new(username: String, private_key_file: PathBuf) -> Self {
        Self {
            username,
            private_key_file,
            timeout: None,
            retries: 0,
        }
    }

    /// Set a timeout duration for the connections.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of times to retries to establish a connection and execute commands.
    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    /// Create a new ssh connection with the provided host.
    pub async fn connect(&self, address: SocketAddr) -> SshResult<SshConnection> {
        SshConnection::new(address, &self.username, self.private_key_file.clone())
            .await
            .map(|x| x.with_timeout(&self.timeout))
    }

    /// Execute the specified ssh command on all provided instances.
    pub async fn execute<'a, I, C>(
        &self,
        instances: I,
        command: &SshCommand<C>,
    ) -> SshResult<Vec<(String, String)>>
    where
        I: Iterator<Item = &'a Instance>,
        C: Fn(usize) -> String + Clone + Send + 'static,
    {
        let handles = instances
            .cloned()
            .enumerate()
            .map(|(i, instance)| {
                let ssh_manager = self.clone();
                let command = command.clone();

                tokio::spawn(async move {
                    let mut error = None;
                    for _ in 0..ssh_manager.retries {
                        let connection = match ssh_manager.connect(instance.ssh_address()).await {
                            Ok(x) => x,
                            Err(e) => {
                                error = Some(e);
                                continue;
                            }
                        };

                        match connection.execute(command.stringify(i)) {
                            r @ Ok(..) => return r,
                            Err(e) => error = Some(e),
                        }
                        sleep(Self::RETRY_DELAY).await;
                    }
                    Err(error.unwrap())
                })
            })
            .collect::<Vec<_>>();

        try_join_all(handles)
            .await
            .unwrap()
            .into_iter()
            .collect::<SshResult<_>>()
    }

    pub async fn wait_for_command<'a, I, C>(
        &self,
        instances: I,
        command: &SshCommand<C>,
        status: CommandStatus,
    ) -> SshResult<()>
    where
        I: Iterator<Item = &'a Instance> + Clone,
        C: Fn(usize) -> String,
    {
        loop {
            sleep(Self::RETRY_DELAY).await;

            let check_command = SshCommand::new(move |_| "(tmux ls || true)".into());
            let result = self.execute(instances.clone(), &check_command).await?;

            if result
                .iter()
                .all(|(stdout, _)| command.status(stdout) == status)
            {
                break;
            }
        }
        Ok(())
    }
}

/// Representation of an ssh connection.
pub struct SshConnection {
    /// The ssh session.
    session: Session,
    /// The host address.
    address: SocketAddr,
}

impl SshConnection {
    /// Default duration before timing out the ssh connection.
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    /// Create a new ssh connection with a specific host.
    pub async fn new<P: AsRef<Path>>(
        address: SocketAddr,
        username: &str,
        private_key_file: P,
    ) -> SshResult<Self> {
        let tcp = TcpStream::connect(address)
            .await
            .map_err(|error| SshError::ConnectionError { address, error })?;

        let mut session =
            Session::new().map_err(|error| SshError::SessionError { address, error })?;
        session.set_timeout(Self::DEFAULT_TIMEOUT.as_millis() as u32);
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|error| SshError::SessionError { address, error })?;
        session
            .userauth_pubkey_file(username, None, private_key_file.as_ref(), None)
            .map_err(|error| SshError::SessionError { address, error })?;

        Ok(Self { session, address })
    }

    /// Set a timeout for the ssh connection. If no timeouts are specified, reset it to the
    /// default value.
    pub fn with_timeout(self, timeout: &Option<Duration>) -> Self {
        let duration = match timeout {
            Some(value) => value,
            None => &Self::DEFAULT_TIMEOUT,
        };
        self.session.set_timeout(duration.as_millis() as u32);
        self
    }

    /// Make a useful session error from the lower level error message.
    fn make_session_error(&self, error: ssh2::Error) -> SshError {
        SshError::SessionError {
            address: self.address,
            error,
        }
    }

    /// Make a useful connection error from the lower level error message.
    fn make_connection_error(&self, error: std::io::Error) -> SshError {
        SshError::ConnectionError {
            address: self.address,
            error,
        }
    }

    /// Execute a ssh command on the remote machine.
    pub fn execute(&self, command: String) -> SshResult<(String, String)> {
        let channel = self
            .session
            .channel_session()
            .map_err(|e| self.make_session_error(e))?;
        self.execute_impl(channel, command)
    }

    /// Execute a ssh command from a given path.
    /// TODO: Eventually remove this function and use [`execute`] through the ssh manager instead.
    pub fn execute_from_path<P: AsRef<Path>>(
        &self,
        command: String,
        path: P,
    ) -> SshResult<(String, String)> {
        let channel = self
            .session
            .channel_session()
            .map_err(|e| self.make_session_error(e))?;
        let command = format!("(cd {} && {command})", path.as_ref().display());
        self.execute_impl(channel, command)
    }

    /// Execute an ssh command on the remote machine and return both stdout and stderr.
    fn execute_impl(&self, mut channel: Channel, command: String) -> SshResult<(String, String)> {
        channel
            .exec(&command)
            .map_err(|e| self.make_session_error(e))?;

        let mut stdout = String::new();
        channel
            .read_to_string(&mut stdout)
            .map_err(|e| self.make_connection_error(e))?;

        let mut stderr = String::new();
        channel
            .stderr()
            .read_to_string(&mut stderr)
            .map_err(|e| self.make_connection_error(e))?;

        channel.close().map_err(|e| self.make_session_error(e))?;
        channel
            .wait_close()
            .map_err(|e| self.make_session_error(e))?;

        let exit_status = channel
            .exit_status()
            .map_err(|e| self.make_session_error(e))?;

        ensure!(
            exit_status == 0,
            SshError::NonZeroExitCode {
                address: self.address,
                code: exit_status,
                message: stderr.clone()
            }
        );

        Ok((stdout, stderr))
    }

    /// Upload a file to the remote machines through scp.
    pub fn upload<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> SshResult<()> {
        let size = content.len() as u64;
        let mut channel = self
            .session
            .scp_send(path.as_ref(), 0o644, size, None)
            .map_err(|e| self.make_session_error(e))?;

        channel
            .write_all(content)
            .map_err(|e| self.make_connection_error(e))?;
        Ok(())
    }

    /// Download a file from the remote machines through scp.
    pub fn download<P: AsRef<Path>>(&self, path: P) -> SshResult<String> {
        let (mut channel, _stats) = self
            .session
            .scp_recv(path.as_ref())
            .map_err(|e| self.make_session_error(e))?;

        let mut content = String::new();
        channel
            .read_to_string(&mut content)
            .map_err(|e| self.make_connection_error(e))?;
        Ok(content)
    }
}