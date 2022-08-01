use clap::Parser;
use color_eyre::eyre::bail;
use color_eyre::eyre::eyre;
use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use itertools::Itertools;
use natspec_parser::NatSpec;
use ropey::Rope;
use serde_json::to_string_pretty;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(clap::Parser)]
#[clap(name = "natspec_parser", author, version, about, long_about = None)]
struct Args {
    #[clap(required = true, multiple_values = true, required = true, value_parser = path_is_file)]
    files: Vec<PathBuf>,
}

fn path_is_file(file_path: &str) -> Result<PathBuf> {
    let p = PathBuf::from(file_path);
    if p.is_file() {
        Ok(p)
    } else {
        bail!("file does not exist")
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let file_names_and_natspecs: Vec<_> =
        args.files.into_iter().map(name_and_natspec).try_collect()?;

    for (file_name, natspecs) in file_names_and_natspecs {
        println!("For file {file_name}, found natspecs:");

        for natspec in natspecs {
            let json = to_string_pretty(&natspec).wrap_err("failed to serialize natspec")?;
            println!("{json}");
        }
    }

    Ok(())
}

fn name_and_natspec(path: PathBuf) -> Result<(String, Vec<NatSpec>)> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| eyre!("could not get file name for {path:?}"))?;

    let file = File::open(&path).wrap_err("file does not exist")?;
    let reader = BufReader::new(file);
    let rope = Rope::from_reader(reader).wrap_err("unable to read file")?;
    let natspecs = NatSpec::from_rope(rope);

    Ok((file_name, natspecs))
}
