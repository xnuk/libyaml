use yaml_rust::parser::{Event, EventReceiver, Parser};

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use yam::argf::argf;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
struct Anchor(usize);

#[derive(Debug, Clone)]
enum Struct {
	Array(Vec<Struct>),
	Map(HashMap<String, Struct>),
	String(String),
}

#[derive(Debug)]
enum EditingStruct {
	Array(Vec<Struct>),
	MapKey(HashMap<String, Struct>, Option<String>),
}

impl EditingStruct {
	fn push(&mut self, item: Struct) -> Option<()> {
		match self {
			EditingStruct::Array(vec) => vec.push(item),
			EditingStruct::MapKey(map, key) => {
				if let Some(k) = key {
					map.insert(k.clone(), item);
					*key = None;
				} else if let Struct::String(scalar) = item {
					*key = Some(scalar);
				} else {
					None?;
				}
			}
		}

		Some(())
	}
}

#[derive(Debug, Default)]
struct Editing {
	anchors: HashMap<Anchor, Struct>,
	anchor_stack: Vec<Anchor>,
	editing_struct: Vec<(Anchor, EditingStruct)>,
	docs: Vec<Struct>,
	failed: bool,
}

impl From<EditingStruct> for Struct {
	fn from(value: EditingStruct) -> Struct {
		match value {
			EditingStruct::Array(a) => Struct::Array(a),
			EditingStruct::MapKey(a, _) => Struct::Map(a),
		}
	}
}

impl Editing {
	fn push_struct(&mut self, anchor: Anchor, s: EditingStruct) {
		self.editing_struct.push((anchor, s));
		self.anchor_stack.push(anchor);
	}

	fn push_new_array(&mut self, anchor: Anchor) {
		self.push_struct(anchor, EditingStruct::Array(Vec::new()));
	}

	fn push_new_map(&mut self, anchor: Anchor) {
		self.push_struct(anchor, EditingStruct::MapKey(HashMap::new(), None));
	}

	fn may_push(&mut self, item: Struct, anchor: Option<Anchor>) -> Option<()> {
		if let Some(anchor) = anchor {
			self.anchors.insert(anchor, item.clone());
		}

		if let Some((_, last)) = self.editing_struct.last_mut() {
			last.push(item)?;
		} else {
			self.docs.push(item);
		}

		Some(())
	}

	fn push(&mut self, item: Struct, anchor: Option<Anchor>) {
		let a = self.may_push(item, anchor);
		self.handle_fail(a);
	}

	fn may_pop(&mut self) -> Option<()> {
		let (anchor, editing) = self.editing_struct.pop()?;
		let item = Struct::from(editing);

		if let Some(last) = self.anchor_stack.last() {
			if *last == anchor {
				self.anchor_stack.pop();
			}
		}

		self.may_push(item, Some(anchor))
	}

	fn pop(&mut self) {
		let a = self.may_pop();
		self.handle_fail(a);
	}

	fn handle_fail(&mut self, action: Option<()>) {
		if action.is_none() {
			self.failed = true;
		}
	}
}

impl EventReceiver for Editing {
	fn on_event(&mut self, ev: Event) {
		use yaml_rust::parser::Event::*;

		match ev {
			// Alias(anchor) => _,
			Scalar(string, style, anchor, tt) => {
				self.push(Struct::String(string), Some(Anchor(anchor)))
			}
			SequenceStart(anchor) => self.push_new_array(Anchor(anchor)),
			MappingStart(anchor) => self.push_new_map(Anchor(anchor)),
			SequenceEnd => self.pop(),
			MappingEnd => self.pop(),
			_ => (),
		}
	}
}

fn main() -> anyhow::Result<()> {
	let bufreader = BufReader::new(argf()?).lines().flat_map(|line| {
		line.unwrap_or_default()
			.chars()
			.chain(['\n'])
			.collect::<Vec<_>>()
	});
	let mut st = Editing::default();
	let parser = Parser::new(bufreader).load(&mut st, false)?;
	println!("{:?}", st);

	Ok(())
}
