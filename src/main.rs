#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate simple_error;
extern crate deunicode;
extern crate itertools;

use deunicode::deunicode;
use simple_error::SimpleError;
use clap::arg_enum;
use chrono::NaiveDate;
use std::fs;
use structopt::StructOpt;
use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::BTreeSet;
use regex::Regex;
use std::str::FromStr;

#[derive(StructOpt)]
#[structopt(about = "Compare theCrag CSV export to manually maintained logbook")]
struct CliArgs {
    #[structopt(long = "thecrag-csv")]
    thecrag_csv: PathBuf,

    #[structopt(long = "logbook")]
    logbook_txt: PathBuf,

    #[structopt(long = "mode", possible_values = &vec!["print", "diff"])]
    mode: OperationMode,
}

arg_enum! {
    enum OperationMode {
        Print,
        Diff,
    }
}

fn main() {
    let args = CliArgs::from_args();

    match args.mode {
        OperationMode::Print => {
            let thecrag_log = generate_logbook_from_thecrag(&args.thecrag_csv);
            match thecrag_log {
                Ok(diff) => println!("{}", diff),
                Err(err) => println!("{}", err),
            };
        }
        OperationMode::Diff => {
            let diff = generate_diff(&args.thecrag_csv, &args.logbook_txt);
            match diff {
                Ok(diff) => print!("{}", diff),
                Err(err) => println!("{}", err),
            };
        }
    }

}

#[derive(Debug)]
struct Tick {
    route_name: String,
    crag_name: String,
    date: NaiveDate,
}

#[derive(Debug)]
struct LogDay {
    date: NaiveDate,
    crags: BTreeSet<String>,
}

type Logbook = BTreeMap<NaiveDate, BTreeSet<String>>;

fn transliterate_crag_name(name: &String) -> String {
    // Manually transliterate umlauts (deunicode doesn't do it)
    let name = name.replace("ä", "ae")
        .replace("ö", "oe")
        .replace("ü", "ue")
        .replace("Ä", "Ae")
        .replace("Ö", "Oe")
        .replace("Ü", "Ue");
    deunicode(&name)
}

fn generate_logbook_from_thecrag(thecrag_csv: &PathBuf) -> Result<String, io::Error> {
    let csv_string = fs::read_to_string(thecrag_csv)?;

    let csv_ticks = get_ticks_from_csv(&csv_string)?;

    let mut days_to_crags: BTreeMap<NaiveDate, BTreeSet<String>> = BTreeMap::new();
    for tick in csv_ticks {
        days_to_crags
            .entry(tick.date)
            .or_insert_with(BTreeSet::new)
            .insert(tick.crag_name);
    }

    Ok(
        days_to_crags
            .iter()
            .map(|(date, crags)| {
                format!(
                    "{}: Felsklettern ({})",
                    date,
                    itertools::join(crags.iter().map(transliterate_crag_name), ", ")
                )
            })
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

fn generate_diff(thecrag_csv: &PathBuf, logbook_txt: &PathBuf) -> Result<String, io::Error> {
    let csv_string = fs::read_to_string(thecrag_csv)?;
    let logbook_string = fs::read_to_string(logbook_txt)?;

    // let csv_ticks = get_ticks_from_csv(&csv_string)?;
    // let logbook_days = get_logbook_from_txt(&logbook_string)?;

    let thecrag_logbook = generate_logbook_from_thecrag(&thecrag_csv)?;
    let txt_logbook = generate_logbook_from_txt(&logbook_txt)?;

    Ok("No diff to report yet".to_string())
}

fn get_crag_name_from_path(crag_path: &str) -> String {
    let mut nodes: Vec<&str> = crag_path.split(" - ").collect();
    let typical_non_crags = vec!["Upper part", "Left", "Right", "Middle", "Centre"];

    let crag_name = loop {
        let last_node = nodes.last().unwrap_or(&"");
        if typical_non_crags.contains(last_node) {
            nodes.pop();
        } else {
            break last_node;
        }
    };

    crag_name.to_string()
}

fn get_ticks_from_csv(csv_string: &str) -> Result<Vec<Tick>, io::Error> {
    let mut csv_reader = csv::Reader::from_reader(csv_string.as_bytes());

    let mut ticks: Vec<Tick> = Vec::new();
    for tick in csv_reader.deserialize() {
        let tick: HashMap<String, String> = tick?;
        let route_name = tick["Ascent Label"].to_string();

        // Note: The "Crag Name" field isn't actually useful (it's always something like
        // "Frankenjura", so we use the last part of the crag path instead).
        let crag_path = tick["Crag Path"].to_string();
        let crag_name = get_crag_name_from_path(&crag_path);

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

fn parse_txt_line(line: &str) -> Result<Option<LogDay>, io::Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^([0-9-]+): Felsklettern \(([^()]+)\)").unwrap();
    }

    let captures = match RE.captures(line) {
        Some(captures) => captures,
        None => return Ok(None),
    };
    let date_str = captures[1].to_string();
    let crags_str = captures[2].to_string();

    let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        Ok(date) => date,
        Err(err) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Cannot parse logbook date \"{}\": {}",
                    date_str,
                    err
                ),
            ));
        }
    };

    let crags: BTreeSet<String> = crags_str.split(", ").map(str::to_string).collect();

    Ok(Some(LogDay { date, crags }))
}

fn get_logbook_from_txt(logbook_string: &str) -> Result<Logbook, io::Error> {
    let logbook_lines = logbook_string
        .split("\n")
        .filter(|line| line.len() > 0)
        .skip_while(|line| *line != "### BEGIN theCrag sync")
        .skip(1)
        .collect::<Vec<&str>>();

    let mut logbook = Logbook::new();
    for line in logbook_lines {
        match parse_txt_line(line)? {
            Some(logday) => { logbook.insert(logday.date, logday.crags); },
            None => {}
        };
    }

    println!("logbook: {:#?}", logbook);

    Ok(Logbook::new())
}
