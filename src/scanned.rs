use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use yaml_rust::scanner::{self, Scanner, TokenType};

use crate::json;

#[inline]
fn empty<T: Default>() -> T {
	Default::default()
}

#[derive(Debug, Clone, PartialEq)]
enum Struct {
	Sequence,
	Mapping,
	IndentlessSequence,
}

#[derive(Debug, Default)]
struct Stack(Vec<Struct>);

impl Stack {
	#[inline]
	fn push(&mut self, item: Struct) {
		self.0.push(item);
	}

	#[inline]
	fn pop(&mut self) -> Option<Struct> {
		self.0.pop()
	}

	#[inline]
	fn last(&self) -> Option<&Struct> {
		self.0.last()
	}
}

#[derive(Debug, Default)]
struct State {
	stack: Stack,
}

impl State {
	#[inline]
	fn push_stack(&mut self, item: Struct) {
		self.stack.push(item)
	}

	#[inline]
	fn pop_stack(&mut self) -> Option<Struct> {
		self.stack.pop()
	}

	#[inline]
	fn last_stack(&self) -> Option<&Struct> {
		self.stack.last()
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Sep {
	StartSeq,
	EndSeq,
	StartMap,
	EndMap,
	Colon,
	Comma,
}

impl Sep {
	fn to_char(&self) -> char {
		use self::Sep::*;

		match self {
			StartSeq => '[',
			StartMap => '{',
			EndSeq => ']',
			EndMap => '}',
			Colon => ':',
			Comma => ',',
		}
	}
}

impl Display for Sep {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		write!(f, "{}", self.to_char())
	}
}

#[derive(Debug, Clone, PartialEq)]
enum StringerItem {
	Char(Sep),
	Scalar(String),
	Anchor(String),
	UseAnchor(String),
}

impl Display for StringerItem {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		use self::StringerItem::*;

		match self {
			Char(sep) => write!(f, "{}", sep.to_char()),
			Scalar(x) => write!(f, "{}", x),
			Anchor(x) => write!(f, "&{}", x),
			UseAnchor(x) => write!(f, "*{}", x),
		}
	}
}

#[derive(Debug, Default)]
struct Stringer {
	data: Vec<StringerItem>,
}

impl Display for Stringer {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		for item in &self.data {
			write!(f, "{}", item)?;
		}

		Ok(())
	}
}

impl Stringer {
	#[inline]
	fn push_sep(&mut self, c: Sep) {
		self.data.push(StringerItem::Char(c))
	}

	#[inline]
	fn push_scalar(&mut self, s: &str) {
		self.data.push(StringerItem::Scalar(json::quote(s)))
	}

	#[inline]
	fn push_anchor(&mut self, s: &str) {
		self.data.push(StringerItem::Anchor(String::from(s)))
	}

	#[inline]
	fn push_use(&mut self, s: &str) {
		self.data.push(StringerItem::UseAnchor(String::from(s)))
	}

	fn add_comma(&mut self) {
		use self::Sep::*;
		use self::StringerItem::*;

		if !matches!(
			self.last(),
			Some(Char(StartMap | StartSeq | Comma | Colon)) | None
		) {
			self.push_sep(Comma);
		}
	}

	#[inline]
	fn last(&self) -> Option<&StringerItem> {
		self.data.last()
	}
}

pub fn parse(buf: impl Iterator<Item = char>) -> anyhow::Result<()> {
	use self::Struct::*;
	use self::TokenType::*;

	let scanner = Scanner::new(buf);
	let mut state = State::default();
	let mut out = Stringer::default();

	for scanner::Token(marker, ptype) in scanner {
		use self::Sep::*;
		if ptype == StreamEnd {
			break;
		}
		let a: () = match &ptype {
			BlockMappingStart | FlowMappingStart => {
				out.push_sep(StartMap);
				state.push_stack(Mapping);
			}
			BlockSequenceStart | FlowSequenceStart => {
				out.push_sep(StartSeq);
				state.push_stack(Sequence)
			}
			BlockEntry | FlowEntry => match state.last_stack().unwrap() {
				Mapping => {
					out.push_sep(StartSeq);
					state.push_stack(IndentlessSequence)
				}
				Sequence | IndentlessSequence => {
					out.add_comma();
				}
			},

			BlockEnd => {
				let mut pop = state.pop_stack().unwrap();
				if matches!(pop, IndentlessSequence) {
					out.push_sep(EndSeq);
					pop = state.pop_stack().unwrap();
				}
				match pop {
					Sequence => out.push_sep(EndSeq),
					Mapping => out.push_sep(EndMap),
					IndentlessSequence => {}
				}
			}
			FlowSequenceEnd => {
				out.push_sep(EndSeq);
				assert_eq!(state.pop_stack(), Some(Sequence));
			}
			FlowMappingEnd => {
				out.push_sep(EndMap);
				assert_eq!(state.pop_stack(), Some(Mapping));
			}

			Key => {
				let last = state.last_stack().unwrap();
				if matches!(last, IndentlessSequence) {
					state.pop_stack();
					out.push_sep(EndSeq);
					// last = state.last_stack().unwrap();
				}

				if let Some(last) = out.last() {
					use self::StringerItem::*;
					match last {
						Char(Comma) => {
							out.push_scalar("");
							out.push_sep(Colon);
							out.push_scalar("");
						}
						Char(Colon) => {
							out.push_scalar("");
						}
						_ => {}
					}
				}

				out.add_comma();
			}
			Value => {
				out.push_sep(Colon);
			}
			Scalar(_, x) => {
				out.push_scalar(x);
			}
			Alias(a) => {
				out.push_use(a);
			}
			Anchor(a) => {
				out.push_anchor(a);
			}

			StreamStart(_)
			| StreamEnd
			| NoToken
			| DocumentStart
			| DocumentEnd
			| VersionDirective(_, _)
			| TagDirective(_, _)
			| Tag(_, _) => (),
		};
		// eprintln!("{:20}     # {:?} / {:?}", out, marker, ptype);
	}
	println!("{}", out);
	Ok(())
}
