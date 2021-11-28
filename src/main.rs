use std::env::args_os;
use std::{fs, io, panic};

use libyaml::yaml::Parser;

fn argf(
) -> io::Result<Box<dyn io::Read + Send + Sync + Unpin + panic::UnwindSafe>> {
	Ok(match args_os().nth(1) {
		Some(path) => Box::new(fs::File::open(path)?),
		None => Box::new(io::stdin()),
	})
}

fn main() -> io::Result<()> {
	let mut file = String::new();
	argf()?.read_to_string(&mut file)?;

	let parser = Parser::new().parse(file.as_str()).collect::<Vec<_>>();

	println!("{:#?}", parser);

	Ok(())
}
