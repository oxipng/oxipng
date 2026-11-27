use std::{
    env,
    error::Error,
    fs::{self, File},
    io::BufWriter,
};

use cargo_metadata::MetadataCommand;
use clap_mangen::Man;
use tz::UtcDateTime;

include!("../../src/cli.rs");

fn main() -> Result<(), Box<dyn Error>> {
    match &*env::args().nth(1).ok_or("No xtask to run provided")? {
        "mangen" => build_manpages(),
        _ => Err("Unknown xtask".into()),
    }
}

fn build_manpages() -> Result<(), Box<dyn Error>> {
    // Put manpages in <working directory>/target/xtask/mangen/manpages. Our working directory is
    // expected to be the root of the repository due to the xtask invocation alias
    let manpages_dir = env::current_dir()?.join("target/xtask/mangen/manpages");
    fs::create_dir_all(&manpages_dir)?;

    let package_meta = MetadataCommand::new()
        .no_deps()
        .exec()?
        .packages
        .into_iter()
        .next()
        .ok_or("missing main package")?;

    // Override the package metadata in the command to that of the main package. Otherwise the
    // metadata is populated from `env!` values that resolve to that of the xtask package due to
    // `include!` evaluation
    let package_cmd = build_command()
        .name(package_meta.name.to_string())
        .version(build_revision().map_or_else(
            || package_meta.version.to_string(),
            |revision| format!("{} ({revision})", package_meta.version),
        ))
        .author(package_meta.authors.first().unwrap_or(&String::new()))
        .about(package_meta.description.unwrap_or_default());

    let mut man_file = BufWriter::new(File::create(manpages_dir.join("oxipng.1"))?);
    Man::new(package_cmd)
        .date({
            let now = UtcDateTime::now()?;
            format!(
                "{:04}-{:02}-{:02}",
                now.year(),
                now.month(),
                now.month_day(),
            )
        })
        .render(&mut man_file)?;

    println!("Manpages generated in {}", manpages_dir.display());

    Ok(())
}
