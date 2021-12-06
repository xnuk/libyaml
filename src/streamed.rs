use std::borrow::Cow;
use std::collections::HashMap;
use yaml_rust::parser::{Event, EventReceiver, Parser};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
struct Anchor(usize);

#[derive(Debug)]
enum TemplateItem<'a> {
	Char(char), // for reduce overhead
	String(Cow<'a, str>),
	Ref(Anchor),
}

#[derive(Debug, Default)]
struct StringHole<'a>(Vec<TemplateItem<'a>>);

#[derive(Debug)]
struct StackItem<'a> {
	anchor: Anchor,
	template: StringHole<'a>,
	depth: isize,
}

#[derive(Debug, Default)]
struct Editing<'a> {
	anchors: HashMap<Anchor, StringHole<'a>>,
	anchor_stack: Vec<(Anchor, StringHole<'a>)>,
}

impl<'a> TryFrom<Event> for TemplateItem<'a> {
	type Error = ();

	fn try_from(event: Event) -> Result<Self, Self::Error> {
		use self::Event::*;
		use self::TemplateItem::*;

		Ok(match event {
			Alias(anchor) => Ref(Anchor(anchor)),
			SequenceStart(_) => Char('['),
			SequenceEnd => Char(']'),
			MappingStart(_) => Char('{'),
			MappingEnd => Char('}'),
			Scalar(string, _, _, _) => String(Cow::from(string)),
			_ => Err(())?,
		})
	}
}

impl<'a> EventReceiver for Editing<'a> {
	fn on_event(&mut self, ev: Event) {}
}

// pub fn parse<T: Iterator<Item = char>>(buf: T) -> anyhow::Result<Editing> {
// 	let mut st = Editing::default();
// 	Parser::new(buf).load(&mut st, false)?;
// 	Ok(st)
// }
