use std::{
    env,
    fs::File,
    io::{BufWriter, Error},
    path::Path,
};

use clap_mangen::Man;

include!("src/cli.rs");

fn build_manpages(outdir: &Path) -> Result<(), Error> {
    let app = build_command();

    let file = Path::new(&outdir).join("oxipng.1");
    let mut file = BufWriter::new(File::create(file)?);

    Man::new(app).render(&mut file)?;

    Ok(())
}

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=src/display_chunks.rs");

    // Create `target/<debug|release>/assets/` folder.
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };
    let out_path = PathBuf::from(outdir);
    let mut path = out_path.ancestors().nth(3).unwrap().to_owned();
    path.push("assets");
    std::fs::create_dir_all(&path).unwrap();

    build_manpages(&path)?;

    Ok(())
}
