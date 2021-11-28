use crate::ffi::{self, yaml_event_type_e as event};

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice::from_raw_parts;

struct Alloc<T> {
	layout: Layout,
	ptr: *mut T,
}

impl<T> Alloc<T> {
	fn new() -> Alloc<T> {
		let layout = Layout::new::<T>();
		let ptr = unsafe { alloc(layout) } as *mut T;

		Alloc { layout, ptr }
	}
}

impl<T> Drop for Alloc<T> {
	fn drop(&mut self) {
		unsafe {
			dealloc(self.ptr as *mut u8, self.layout);
		}
	}
}

#[derive(Debug)]
pub enum Token {
	StreamStart,
	StreamEnd,
	DocumentStart,
	DocumentEnd,
	Alias {
		anchor: String,
	},
	Scalar {
		anchor: Option<String>,
		value: String,
	},
	SequenceStart {
		anchor: Option<String>,
	},
	SequenceEnd,
	MappingStart {
		anchor: Option<String>,
	},
	MappingEnd,
}

unsafe fn unwrap_cstr(ptr: *const c_char) -> Option<String> {
	if ptr.is_null() {
		return None;
	}

	Some(String::from(CStr::from_ptr(ptr).to_str().ok()?))
}

unsafe fn unwrap_string_sized(
	ptr: *const ffi::yaml_char_t,
	len: usize,
) -> Option<String> {
	if ptr.is_null() {
		return None;
	}

	let slice = from_raw_parts(ptr, len).to_vec();
	String::from_utf8(slice).ok()
}

unsafe fn unwrap_string(ptr: *const ffi::yaml_char_t) -> Option<String> {
	if ptr.is_null() {
		return None;
	}

	let mut len = 0;
	while *(ptr.add(len)) != 0 {
		len += 1;
	}

	unwrap_string_sized(ptr, len)
}

impl TryFrom<ffi::yaml_event_s> for Token {
	type Error = ();
	fn try_from(event: ffi::yaml_event_s) -> Result<Self, Self::Error> {
		use self::Token::*;

		match event.type_ {
			event::YAML_STREAM_START_EVENT => Ok(StreamStart),
			event::YAML_STREAM_END_EVENT => Ok(StreamEnd),
			event::YAML_DOCUMENT_START_EVENT => Ok(DocumentStart),
			event::YAML_DOCUMENT_END_EVENT => Ok(DocumentEnd),
			event::YAML_ALIAS_EVENT => {
				let anchor = unsafe { unwrap_string(event.data.alias.anchor) }
					.ok_or(())?;
				Ok(Alias { anchor })
			}
			event::YAML_SCALAR_EVENT => {
				let scalar = unsafe { event.data.scalar };

				let anchor = unsafe { unwrap_string(scalar.anchor) };
				let value = unsafe {
					unwrap_string_sized(scalar.value, scalar.length as usize)
				}
				.ok_or(())?;

				Ok(Scalar { anchor, value })
			}
			event::YAML_SEQUENCE_START_EVENT => {
				let anchor =
					unsafe { unwrap_string(event.data.sequence_start.anchor) };
				Ok(SequenceStart { anchor })
			}
			event::YAML_SEQUENCE_END_EVENT => Ok(SequenceEnd),
			event::YAML_MAPPING_START_EVENT => {
				let anchor =
					unsafe { unwrap_string(event.data.mapping_start.anchor) };
				Ok(MappingStart { anchor })
			}
			event::YAML_MAPPING_END_EVENT => Ok(MappingEnd),

			_ => Err(()),
		}
	}
}

struct Event {
	alloc: Alloc<ffi::yaml_event_t>,
}

impl Event {
	fn empty() -> Event {
		Event {
			alloc: Alloc::new(),
		}
	}
}

impl Drop for Event {
	fn drop(&mut self) {
		unsafe {
			ffi::yaml_event_delete(self.alloc.ptr);
		}
	}
}

pub struct Parser {
	alloc: Alloc<ffi::yaml_parser_t>,
	string: Option<CString>,
}

impl Parser {
	#[must_use]
	pub fn new() -> Parser {
		let alloc = Alloc::new();

		unsafe {
			ffi::yaml_parser_initialize(alloc.ptr);
		};

		Parser {
			alloc,
			string: None,
		}
	}

	pub fn parse(&mut self, source: &str) -> ParserIter {
		let parser = self.alloc.ptr;
		let raw = CString::new(source).unwrap();

		unsafe {
			ffi::yaml_parser_set_input_string(
				parser,
				raw.as_ptr() as *const u8,
				raw.as_bytes().len() as u64,
			)
		}

		// moving
		self.string = Some(raw);

		ParserIter {
			parser: self,
			done: false,
		}
	}
}

impl Drop for Parser {
	fn drop(&mut self) {
		unsafe {
			ffi::yaml_parser_delete(self.alloc.ptr);
		}
	}
}

pub struct ParserIter<'a> {
	parser: &'a mut Parser,
	done: bool,
}

impl<'a> Iterator for ParserIter<'a> {
	type Item = Token;

	fn next(&mut self) -> Option<Self::Item> {
		if self.done {
			return None;
		}

		let event = Event::empty();
		let has_next = unsafe {
			ffi::yaml_parser_parse(self.parser.alloc.ptr, event.alloc.ptr) != 0
		};

		if !has_next {
			self.done = true;

			let error =
				unsafe { unwrap_cstr((*self.parser.alloc.ptr).problem) };

			if let Some(error) = error {
				eprintln!("YAML parsing error: {}", error);
			}

			return None;
		}

		let event = unsafe { *event.alloc.ptr };
		let token = Token::try_from(event).ok();

		match token {
			Some(Token::StreamEnd) => {
				self.done = true;
				token
			}
			Some(_) => token,
			None => self.next(),
		}
	}
}
