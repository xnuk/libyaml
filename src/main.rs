use yam::argf::argf_chars;
use yam::streamed::parse;

fn main() -> anyhow::Result<()> {
	let r = parse(argf_chars()?)?;
	for v in r {
		print!("{}", v);
	}
	Ok(())
}
