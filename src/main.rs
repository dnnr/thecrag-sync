use chrono::NaiveDate;
use std::fs;
use structopt::StructOpt;
use std::io;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(StructOpt)]
#[structopt(about = "Compare theCrag CSV export to manually maintained logbook")]
struct CliArgs {
    #[structopt(long = "csv")]
    csv_file: PathBuf,

    #[structopt(long = "logbook")]
    logbook_file: PathBuf,
}

fn main() {
    let args = CliArgs::from_args();

    let diff = generate_diff(&args.csv_file, &args.logbook_file);

    match diff {
        Ok(diff) => print!("{}", diff),
        Err(err) => println!("{}", err),
    };
}

#[derive(Debug)]
struct Tick {
    route_name: String,
    crag_name: String,
    date: NaiveDate,
}

#[derive(Debug)]
struct LogDay {
    crag_name: String,
    date: NaiveDate,
}

fn generate_diff(csv_file: &PathBuf, logbook_file: &PathBuf) -> Result<String, io::Error> {
    let csv_string = fs::read_to_string(csv_file)?;
    let logbook_string = fs::read_to_string(logbook_file)?;

    let csv_ticks = get_ticks_from_csv(&csv_string);
    let logbook_days = get_logdays_from_logbook(&logbook_string);

    println!("{:?}", csv_ticks?);

    Ok("No diff to report yet".to_string())
}

// fn read_csv(path: &str) -> Result<String, io::Error> {
// }

fn get_ticks_from_csv(csv_string: &str) -> Result<Vec<Tick>, io::Error> {
    let mut csv_reader = csv::Reader::from_reader(csv_string.as_bytes());

    let mut ticks: Vec<Tick> = Vec::new();
    for tick in csv_reader.deserialize() {
        let tick: HashMap<String, String> = tick?;
        let route_name = tick["Ascent Label"].to_string();
        let crag_name = tick["Crag Name"].to_string();

        let date_str = &tick["Ascent Date"];
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ");
        let date = match date {
            Ok(date) => date,
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Cannot parse date field \"{}\": {}", date_str, err),
                ))
            }
        };

        ticks.push(Tick {
            route_name,
            crag_name,
            date,
        });
    }

    Ok(ticks)
}

fn get_logdays_from_logbook(logbook_string: &str) -> Result<Vec<LogDay>, io::Error> {
    Ok(vec![])
}
