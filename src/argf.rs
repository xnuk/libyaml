use std::env::args_os;
use std::io::{BufRead, BufReader};
use std::{fs, io, panic};

pub fn argf(
) -> io::Result<Box<dyn io::Read + Send + Sync + Unpin + panic::UnwindSafe>> {
	Ok(match args_os().nth(1) {
		Some(path) => Box::new(fs::File::open(path)?),
		None => Box::new(io::stdin()),
	})
}

pub fn argf_chars() -> io::Result<impl Iterator<Item = char>> {
	Ok(BufReader::new(argf()?)
		.lines()
		.flat_map(|line| -> Vec<char> {
			line.unwrap_or_default().chars().chain(['\n']).collect()
		}))
}
