use yaml_rust::parser::{Event, EventReceiver, Parser};

use std::io::{BufRead, BufReader};
use std::iter::empty;
use yam::argf::argf;

struct Recv;

impl EventReceiver for Recv {
	fn on_event(&mut self, ev: Event) {
		println!("{:?}", ev);
	}
}

fn main() -> anyhow::Result<()> {
	let parser =
		Parser::new(BufReader::new(argf()?).lines().flat_map(|line| {
			line.unwrap_or_default()
				.chars()
				.chain(['\n'])
				.collect::<Vec<_>>()
		}))
		.load(&mut Recv, false)?;

	Ok(())
}
