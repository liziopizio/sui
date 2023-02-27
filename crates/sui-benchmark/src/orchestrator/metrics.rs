use std::{collections::HashMap, fs, hash::Hash, io::BufRead, path::PathBuf, time::Duration};

use num_integer::Roots;
use prettytable::{format, row, Table};
use prometheus_parse::Scrape;
use serde::Serialize;

use super::BenchmarkParameters;

type BucketId = String;

#[derive(Serialize, Default)]
pub struct DataPoint {
    /// Duration since the beginning of the benchmark.
    timestamp: Duration,
    /// Latency buckets.
    buckets: HashMap<BucketId, usize>,
    /// Sum of the latencies of all finalized transactions.
    sum: Duration,
    /// Total number of finalized transactions
    count: usize,
    /// Square of the latencies of all finalized transactions.
    squared_sum: Duration,
}

impl DataPoint {
    pub fn new(
        benchmark_timestamp: Duration,
        buckets: HashMap<BucketId, usize>,
        sum: Duration,
        count: usize,
        squared_sum: Duration,
    ) -> Self {
        Self {
            timestamp: benchmark_timestamp,
            buckets,
            sum,
            count,
            squared_sum,
        }
    }

    pub fn tps(&self) -> u64 {
        let tps = self.count.checked_div(self.timestamp.as_secs() as usize);
        tps.unwrap_or_default() as u64
    }

    pub fn average_latency(&self) -> Duration {
        let latency_in_millis = self.sum.as_millis().checked_div(self.count as u128);
        Duration::from_millis(latency_in_millis.unwrap_or_default() as u64)
    }

    pub fn stdev_latency(&self) -> Duration {
        // stdev = sqrt( squared_sum / count - avg^2 )
        let first_term = self.squared_sum.as_millis().checked_div(self.count as u128);
        let squared_avg = self.average_latency().as_millis().pow(2);
        let variance = first_term.unwrap_or_default().checked_sub(squared_avg);
        let stdev = variance.unwrap_or_default().sqrt();
        Duration::from_millis(stdev as u64)
    }

    /// Aggregate the benchmark duration of multiple data points by taking the max.
    pub fn aggregate_duration(data_points: &[&Self]) -> Duration {
        data_points
            .iter()
            .map(|x| x.timestamp)
            .max()
            .unwrap_or_default()
    }

    /// Aggregate the tps of multiple data points by taking the sum.
    pub fn aggregate_tps(data_points: &[&Self]) -> u64 {
        data_points.iter().map(|x| x.tps()).sum()
    }

    /// Aggregate the average latency of multiple data points by taking the average.
    pub fn aggregate_average_latency(data_points: &[&Self]) -> Duration {
        Duration::from_millis(
            data_points
                .iter()
                .map(|x| x.average_latency().as_millis())
                .sum::<u128>()
                .checked_div(data_points.len() as u128)
                .unwrap_or_default() as u64,
        )
    }

    /// Aggregate the stdev latency of multiple data points by taking the max.
    pub fn aggregate_stdev_latency(data_points: &[&Self]) -> Duration {
        data_points
            .iter()
            .map(|x| x.stdev_latency())
            .max()
            .unwrap_or_default()
    }
}

#[derive(Serialize)]
pub struct MetricsCollector<ScraperId: Serialize> {
    parameters: BenchmarkParameters,
    scrapers: HashMap<ScraperId, Vec<DataPoint>>,
}

impl<ScraperId> MetricsCollector<ScraperId>
where
    ScraperId: Eq + Hash + Serialize,
{
    pub fn new(parameters: BenchmarkParameters) -> Self {
        Self {
            parameters,
            scrapers: HashMap::new(),
        }
    }

    pub fn collect(&mut self, scraper_id: ScraperId, text: &str) {
        let br = std::io::BufReader::new(text.as_bytes());
        let parsed = Scrape::parse(br.lines()).unwrap();

        let buckets: HashMap<_, _> = parsed
            .samples
            .iter()
            .find(|x| x.metric == "latency_s")
            .map(|x| match &x.value {
                prometheus_parse::Value::Histogram(values) => values
                    .iter()
                    .map(|x| {
                        let bucket_id = x.less_than.to_string();
                        let count = x.count as usize;
                        (bucket_id, count)
                    })
                    .collect(),
                _ => panic!("Unexpected scraped value"),
            })
            .unwrap_or_default();

        let sum = parsed
            .samples
            .iter()
            .find(|x| x.metric == "latency_s_sum")
            .map(|x| match x.value {
                prometheus_parse::Value::Untyped(value) => Duration::from_secs(value as u64),
                _ => panic!("Unexpected scraped value"),
            })
            .unwrap_or_default();

        let count = parsed
            .samples
            .iter()
            .find(|x| x.metric == "latency_s_count")
            .map(|x| match x.value {
                prometheus_parse::Value::Untyped(value) => value as usize,
                _ => panic!("Unexpected scraped value"),
            })
            .unwrap_or_default();

        let squared_sum = parsed
            .samples
            .iter()
            .find(|x| x.metric == "latency_squared_s")
            .map(|x| match x.value {
                prometheus_parse::Value::Counter(value) => Duration::from_secs(value as u64),
                _ => panic!("Unexpected scraped value"),
            })
            .unwrap_or_default();

        let duration = parsed
            .samples
            .iter()
            .find(|x| x.metric == "benchmark_duration")
            .map(|x| match x.value {
                prometheus_parse::Value::Counter(value) => Duration::from_secs(value as u64),
                _ => panic!("Unexpected scraped value"),
            })
            .unwrap_or_default();

        self.scrapers
            .entry(scraper_id)
            .or_insert_with(Vec::new)
            .push(DataPoint::new(duration, buckets, sum, count, squared_sum));
    }

    pub fn save(&self) {
        let json = serde_json::to_string(self).expect("Cannot serialize metrics");
        let path = PathBuf::from("results.json");
        fs::write(path, json).unwrap();
    }

    pub fn print_summary(&self, parameters: &BenchmarkParameters) {
        let last_data_points: Vec<_> = self.scrapers.values().filter_map(|x| x.last()).collect();
        let duration = DataPoint::aggregate_duration(&last_data_points);
        let total_tps = DataPoint::aggregate_tps(&last_data_points);
        let average_latency = DataPoint::aggregate_average_latency(&last_data_points);
        let stdev_latency = DataPoint::aggregate_stdev_latency(&last_data_points);

        let mut table = Table::new();
        let format = format::FormatBuilder::new()
            .separators(
                &[
                    format::LinePosition::Top,
                    format::LinePosition::Bottom,
                    format::LinePosition::Title,
                ],
                format::LineSeparator::new('-', '-', '-', '-'),
            )
            .padding(1, 1)
            .build();
        table.set_format(format);

        println!();
        table.set_titles(row![bH2->"Benchmark Summary"]);
        table.add_row(row![b->"Nodes:", parameters.nodes]);
        table.add_row(row![b->"Faults:", parameters.faults]);
        table.add_row(row![b->"Load:", format!("{} tx/s", parameters.load)]);
        table.add_row(row![b->"Duration:", format!("{} s", duration.as_secs())]);
        table.add_row(row![bH2->""]);
        table.add_row(row![b->"TPS:", format!("{total_tps} tx/s")]);
        table.add_row(row![b->"Latency (avg):", format!("{} ms", average_latency.as_millis())]);
        table.add_row(row![b->"Latency (stdev):", format!("{} ms", stdev_latency.as_millis())]);
        table.printstd();
        println!();
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, time::Duration};

    use super::{BenchmarkParameters, DataPoint, MetricsCollector};

    #[test]
    fn average_latency() {
        let data = DataPoint::new(
            Duration::from_secs(10), // benchmark_timestamp
            HashMap::new(),          // buckets
            Duration::from_secs(2),  // sum
            100,                     // count
            Duration::from_secs(0),  // squared_sum
        );

        assert_eq!(data.average_latency(), Duration::from_millis(20));
    }

    #[test]
    fn stdev_latency() {
        let data = DataPoint::new(
            Duration::from_secs(10),  // benchmark_timestamp
            HashMap::new(),           // buckets
            Duration::from_secs(2),   // sum
            100,                      // count
            Duration::from_secs(290), // squared_sum
        );

        // squared_sum / count
        assert_eq!(data.squared_sum.as_millis() / data.count as u128, 2900);
        // avg^2
        assert_eq!(data.average_latency().as_millis().pow(2), 400);
        // sqrt( squared_sum / count - avg^2 )
        assert_eq!(data.stdev_latency(), Duration::from_millis(50));
    }

    #[test]
    fn collect() {
        let report = r#"
            # HELP benchmark_duration Duration of the benchmark
            # TYPE benchmark_duration counter
            benchmark_duration 30
            # HELP latency_s Total time in seconds to return a response
            # TYPE latency_s histogram
            latency_s_bucket{workload=transfer_object,le=0.1} 0
            latency_s_bucket{workload=transfer_object,le=0.25} 0
            latency_s_bucket{workload=transfer_object,le=0.5} 506
            latency_s_bucket{workload=transfer_object,le=0.75} 1282
            latency_s_bucket{workload=transfer_object,le=1} 1693
            latency_s_bucket{workload="transfer_object",le="1.25"} 1816
            latency_s_bucket{workload="transfer_object",le="1.5"} 1860
            latency_s_bucket{workload="transfer_object",le="1.75"} 1860
            latency_s_bucket{workload="transfer_object",le="2"} 1860
            latency_s_bucket{workload=transfer_object,le=2.5} 1860
            latency_s_bucket{workload=transfer_object,le=5} 1860
            latency_s_bucket{workload=transfer_object,le=10} 1860
            latency_s_bucket{workload=transfer_object,le=20} 1860
            latency_s_bucket{workload=transfer_object,le=30} 1860
            latency_s_bucket{workload=transfer_object,le=60} 1860
            latency_s_bucket{workload=transfer_object,le=90} 1860
            latency_s_bucket{workload=transfer_object,le=+Inf} 1860
            latency_s_sum{workload=transfer_object} 1265.287933130998
            latency_s_count{workload=transfer_object} 1860
            # HELP latency_squared_s Square of total time in seconds to return a response
            # TYPE latency_squared_s counter
            latency_squared_s{workload="transfer_object"} 952.8160642745289
        "#;

        let mut aggregator = MetricsCollector::new(BenchmarkParameters::default());
        let scraper_id = 1u8;
        aggregator.collect(scraper_id, report);

        assert_eq!(aggregator.scrapers.len(), 1);
        let data_points = aggregator.scrapers.get(&scraper_id).unwrap();
        assert_eq!(data_points.len(), 1);

        let data = &data_points[0];
        assert_eq!(
            data.buckets,
            ([
                ("0.1".into(), 0),
                ("0.25".into(), 0),
                ("0.5".into(), 506),
                ("0.75".into(), 1282),
                ("1".into(), 1693),
                ("1.25".into(), 1816),
                ("1.5".into(), 1860),
                ("1.75".into(), 1860),
                ("2".into(), 1860),
                ("2.5".into(), 1860),
                ("5".into(), 1860),
                ("10".into(), 1860),
                ("20".into(), 1860),
                ("30".into(), 1860),
                ("60".into(), 1860),
                ("90".into(), 1860),
                ("inf".into(), 1860)
            ])
            .iter()
            .cloned()
            .collect()
        );
        assert_eq!(data.sum, Duration::from_secs(1265));
        assert_eq!(data.count, 1860);
        assert_eq!(data.timestamp, Duration::from_secs(30));
        assert_eq!(data.squared_sum, Duration::from_secs(952));

        assert_eq!(data.tps(), 62);
        assert_eq!(data.average_latency(), Duration::from_millis(680));
        assert_eq!(data.stdev_latency(), Duration::from_millis(680));
    }
}