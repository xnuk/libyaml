fn escape_char(c: char) -> Option<String> {
	Some(match c {
		'\x08' => "\\b".to_owned(),
		'\t' => "\\t".to_owned(),
		'\n' => "\\n".to_owned(),
		'\x0c' => "\\f".to_owned(),
		'\r' => "\\r".to_owned(),
		'"' => "\\\"".to_owned(),
		'\\' => "\\\\".to_owned(),
		_ => {
			let num = c as u32;
			if num < (u16::MAX as u32) && c.is_control() {
				format!("\\u{:04x}", num as u16)
			} else {
				None?
			}
		}
	})
}

pub fn quote(s: &str) -> String {
	let mut ss = s
		.chars()
		.flat_map(|c| match escape_char(c) {
			Some(s) => s.chars().collect(),
			None => Vec::from([c]),
		})
		.collect::<String>();

	ss.insert(0, '"');
	ss.push('"');
	ss
}

#[cfg(test)]
mod test {
	use super::quote;

	macro_rules! str {
		($($s:literal)+) => { [$($s),+].iter().collect() }
	}

	#[test]
	fn escape_test() {
		let (input, output): (String, String) = (
			str!('\\' 's' 'e' 'x' '"' '\\' '\t'),
			str!('"' '\\' '\\' 's' 'e' 'x' '\\' '"' '\\' '\\' '\\' 't' '"'),
		);

		assert_eq!(quote(&input), output);
	}
}
