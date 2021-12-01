pub mod argf {
	use std::env::args_os;
	use std::{fs, io, panic};

	pub fn argf(
	) -> io::Result<Box<dyn io::Read + Send + Sync + Unpin + panic::UnwindSafe>>
	{
		Ok(match args_os().nth(1) {
			Some(path) => Box::new(fs::File::open(path)?),
			None => Box::new(io::stdin()),
		})
	}
}
