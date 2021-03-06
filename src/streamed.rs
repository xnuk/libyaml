use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use yaml_rust::parser::{Event, EventReceiver, Parser};

use crate::json;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Anchor(usize);

#[derive(Debug, Clone)]
enum TemplateItem<'a> {
	Char(char), // for reduce overhead
	String(Cow<'a, str>),
	Ref(Anchor),
}

#[derive(Debug, Default, Clone)]
struct StringHole<'a> {
	data: Vec<TemplateItem<'a>>,
	len: usize,
}

#[derive(Debug, Default, Clone)]
pub struct Hole(VecDeque<Result<String, Anchor>>);

impl<'a> StringHole<'a> {
	fn singleton(a: TemplateItem<'a>) -> StringHole<'a> {
		StringHole {
			data: vec![a],
			len: 0,
		}
	}

	fn prepend_comma(&mut self) -> bool {
		use self::TemplateItem::*;

		match self.data.last() {
			None | Some(Char('{')) | Some(Char('[')) => false,
			_ => match self.data[0] {
				Char('[') => {
					self.data.push(Char(','));
					true
				}
				Char('{') => {
					self.data.push(if self.len % 2 == 0 {
						Char(':')
					} else {
						Char(',')
					});
					true
				}
				_ => false,
			},
		}
	}

	fn push(&mut self, item: TemplateItem<'a>) {
		use self::TemplateItem::*;

		if !matches!(item, Char(_)) {
			self.prepend_comma();
		}

		self.data.push(item);
		self.len += 1;
	}

	fn append(&mut self, item: &mut StringHole<'a>) {
		self.prepend_comma();
		self.data.append(&mut item.data);
		self.len += 1;
	}

	fn fold(&self) -> Hole {
		let mut res = VecDeque::new();
		let mut st = String::new();

		for item in &self.data {
			match item {
				TemplateItem::Char(c) => {
					st.push(*c);
				}
				TemplateItem::String(s) => {
					st.push_str(s);
				}
				TemplateItem::Ref(r) => {
					res.push_back(Ok(st));
					res.push_back(Err(*r));
					st = String::new();
				}
			}
		}

		if !st.is_empty() {
			res.push_back(Ok(st));
		}

		Hole(res)
	}
}

#[derive(Debug)]
struct StackItem<'a> {
	anchor: Anchor,
	template: StringHole<'a>,
}

impl<'a> StackItem<'a> {
	fn new(anchor: Anchor) -> StackItem<'a> {
		StackItem {
			anchor,
			template: StringHole::default(),
		}
	}

	fn push(&mut self, item: TemplateItem<'a>) {
		self.template.push(item)
	}
}

#[derive(Debug)]
pub enum EditingError {
	Syntax,
	RecursiveRef,
}

impl Display for EditingError {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		use self::EditingError::*;

		match self {
			Syntax => write!(f, "Unexpected end of document"),
			RecursiveRef => write!(f, "Recursive reference has been found"),
		}
	}
}

impl Error for EditingError {}

#[derive(Debug, Default)]
struct Editing<'a> {
	anchors: HashMap<Anchor, StringHole<'a>>,
	anchor_stack: Vec<StackItem<'a>>,
	finished: Option<Result<Anchor, EditingError>>,
}

impl Editing<'_> {
	fn fold_up(&mut self) -> Option<()> {
		if let Some(mut a) = self.anchor_stack.pop() {
			self.anchors.insert(a.anchor, a.template.clone());
			if let Some(last) = self.anchor_stack.last_mut() {
				last.template.append(&mut a.template);
			} else {
				self.finished = Some(Ok(a.anchor));
			}
		}

		Some(())
	}
}

#[derive(Debug, Clone)]
struct Folded {
	anchors: HashMap<Anchor, Hole>,
	starts: Anchor,
}

impl Folded {
	fn iter(self) -> Option<FoldedIter> {
		let anchors = self.anchors;
		Some(FoldedIter {
			queue: anchors.get(&self.starts)?.clone(),
			anchors,
		})
	}
}

pub struct FoldedIter {
	anchors: HashMap<Anchor, Hole>,
	queue: Hole,
}

impl Iterator for FoldedIter {
	type Item = String;

	fn next(&mut self) -> Option<Self::Item> {
		match self.queue.0.pop_front().as_ref()? {
			Ok(s) => Some(s.clone()),
			Err(anchor) => {
				let mut hole = self.anchors.get(anchor)?.clone();
				hole.0.append(&mut self.queue.0);
				self.queue = hole;
				self.next()
			}
		}
	}
}

impl TryFrom<Editing<'_>> for Folded {
	type Error = EditingError;

	fn try_from(editing: Editing) -> Result<Self, Self::Error> {
		if let Some(starts) = editing.finished {
			let starts = starts?;
			let anchors = editing
				.anchors
				.iter()
				.map(|(k, v)| (*k, v.fold()))
				.collect();

			Ok(Folded { anchors, starts })
		} else {
			Err(EditingError::Syntax)
		}
	}
}

impl TryFrom<&Event> for TemplateItem<'_> {
	type Error = ();

	fn try_from(event: &Event) -> Result<Self, Self::Error> {
		use self::Event::*;
		use self::TemplateItem::*;

		Ok(match event {
			Alias(anchor) => Ref(Anchor(*anchor)),
			SequenceStart(_) => Char('['),
			SequenceEnd => Char(']'),
			MappingStart(_) => Char('{'),
			MappingEnd => Char('}'),
			Scalar(string, _, _, _) => String(Cow::from(json::quote(string))),
			_ => return Err(()),
		})
	}
}

#[derive(Debug, Copy, Clone)]
enum StructBoundary {
	Start,
	End,
}

impl TryFrom<&Event> for StructBoundary {
	type Error = ();

	fn try_from(event: &Event) -> Result<Self, Self::Error> {
		use self::Event::*;
		use self::StructBoundary::*;

		Ok(match event {
			SequenceStart(_) | MappingStart(_) => Start,
			SequenceEnd | MappingEnd => End,
			_ => return Err(()),
		})
	}
}

impl TryFrom<&Event> for Anchor {
	type Error = ();

	fn try_from(event: &Event) -> Result<Self, Self::Error> {
		use self::Event::*;

		match event {
			SequenceStart(x) | MappingStart(x) | Scalar(_, _, x, _) => {
				Ok(Anchor(*x))
			}
			_ => Err(()),
		}
	}
}

impl EventReceiver for Editing<'_> {
	fn on_event(&mut self, ev: Event) {
		use self::Event::*;

		if let Some(Err(_)) = self.finished {
			return;
		}

		if let Ok(item) = TemplateItem::try_from(&ev) {
			if let Ok(anchor) = Anchor::try_from(&ev) {
				match ev {
					Scalar(_, _, _, _) => {
						self.anchors.insert(
							anchor,
							StringHole::singleton(item.clone()),
						);
					}
					SequenceStart(_) | MappingStart(_) => {
						self.anchor_stack.push(StackItem::new(anchor));
					}
					_ => {}
				}
			}

			if let TemplateItem::Ref(r) = item {
				if self.anchor_stack.iter().any(|x| x.anchor == r) {
					self.finished = Some(Err(EditingError::RecursiveRef));
					return;
				}
			}

			let last = self.anchor_stack.last_mut();

			if let Some(last) = last {
				last.push(item);

				if matches!(ev, SequenceEnd | MappingEnd) {
					self.fold_up();
				}
			}
		}
	}
}

pub fn parse<T: Iterator<Item = char>>(buf: T) -> anyhow::Result<FoldedIter> {
	let mut st = Editing::default();
	Parser::new(buf).load(&mut st, false)?;
	let st = Folded::try_from(st)?.iter().ok_or(EditingError::Syntax)?;
	Ok(st)
}

#[cfg(test)]
mod test {
	#[test]
	fn parsing_test() {
		let m = include_str!("../data/bomb.yaml");
		super::parse(m.chars()).unwrap();
	}
}
