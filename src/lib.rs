#[macro_use]
extern crate lazy_static;

extern crate regex;

mod tests;

pub struct Message {
  pub name:         String,
  pub full_name:    String,
  pub message_type: MessageType,
  pub number:       u8,
  pub fields:       Vec<Field>
}

#[derive(Copy, Clone)]
pub enum MessageType {
  Request,
  Response
}

impl MessageType {
  fn char(self) -> char {
    match self {
      MessageType::Request => 'T',
      MessageType::Response => 'R',
    }
  }
}

pub struct Field {
  pub name:       String,
  pub times:      Option<String>,
  pub field_type: FieldType
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum FieldType {
  U8,
  U16,
  U32,
  U64,
  QID,
  Stat,
  Bytes,
  String,
}

impl FieldType {
  fn integer(self) -> bool {
    match self {
      FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::U64 => true,
      _ => false,
    }
  }
}

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    try!(write!(f, "size[4] {}:{} tag[2]", self.full_name, self.number));
    for field in &self.fields {
      try!(write!(f, " {}", field))
    }
    Ok(())
  }
}

impl std::fmt::Display for Field {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self.times {
      Some(ref times) => write!(f, "{}*({}[{}])", times, self.name, self.field_type),
      None => write!(f, "{}[{}]", self.name, self.field_type)
    }
  }
}

impl std::fmt::Display for FieldType {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let s = match self {
      &FieldType::U8 => "1",
      &FieldType::U16 => "2",
      &FieldType::U32 => "4",
      &FieldType::U64 => "8",
      &FieldType::QID => "13",
      &FieldType::Stat => "n",
      &FieldType::Bytes => "count",
      &FieldType::String => "s"
    };
    write!(f, "{}", s)
  }
}

macro_rules! re {
  ($name:ident $pattern:expr) => (
    lazy_static! {
      static ref $name: regex::Regex = regex::Regex::new($pattern).unwrap();
    }
  )
}

fn field(input: &str) -> (&str, Option<Field>) {
  re!{ SCALAR r"^\s*([a-z]+)\s*[[](1|2|4|8|13|count|n|s)]\s*" }
  re!{ ARRAY r"^\s*([a-z]+)\s*[*]\s*[(]([^)]*)[)]\s*" }
  if let Some(captures) = SCALAR.captures(input) {
    let name = captures.at(1).unwrap();
    let field_type = match captures.at(2).unwrap() {
      "1"     => FieldType::U8,
      "2"     => FieldType::U16,
      "4"     => FieldType::U32,
      "8"     => FieldType::U64,
      "13"    => FieldType::QID,
      "count" => FieldType::Bytes,
      "n"     => FieldType::Stat,
      "s"     => FieldType::String,
      _       => return (input, None),
    };
    let rest = &input[captures.at(0).unwrap().len()..];
    (rest, Some(Field{name: name.to_owned(), times: None, field_type: field_type}))
  } else if let Some(captures) = ARRAY.captures(input) {
    let times = captures.at(1).unwrap();
    let field_text = captures.at(2).unwrap();
    let rest = &input[captures.at(0).unwrap().len()..];
    if let (_, Some(mut field)) = field(field_text) {
      field.times = Some(times.to_owned());
      (rest, Some(field))
    } else {
      (input, None)
    }
  } else {
    (input, None)
  }
}

fn name(input: &str) -> (&str, Option<(MessageType, &str, u8)>) {
  re!(NAME r"\s*(T|R)([a-z]+)\s*:\s*([1-9][0-9]*)\s*");
  if let Some(captures) = NAME.captures(input) {
    let message_type = match captures.at(1).unwrap() {
      "T" => MessageType::Request,
      "R" => MessageType::Response,
      _ => return (input, None),
    };
    let name = captures.at(2).unwrap();
    let number = captures.at(3).unwrap().parse().unwrap();
    let rest = &input[captures.at(0).unwrap().len()..];
    (rest, Some((message_type, name, number)))
  } else {
    (input, None)
  }
}

fn error(line_number: usize, line: &str, message: &str) -> Result<Vec<Message>, String> {
  Err(if line_number > 0 {
    format!("line {}: {}\nerror: {}", line_number, line, message)
  } else {
    format!("error: {}", message)
  })
}

pub fn strip(definition: &str) -> Vec<(usize, &str)> {
  definition.lines().enumerate().flat_map(|(i, mut line)| {
    if let Some(i) = line.find('#') {
      line = &line[0..i]
    }

    line = line.trim();

    if line.len() == 0 {
      None
    } else {
      Some((i, line))
    }
  }).collect()
}

pub fn parse(definition: &str) -> Result<Vec<Message>, String> {
  let mut messages = vec![];
  let mut names = std::collections::HashSet::<String>::new();
  let mut numbers = std::collections::HashSet::<u8>::new();

  for (i, line) in strip(definition) {
    let n = i + 1;
    let (s, size) = field(line);
    match size {
      Some(field) => if field.name != "size" || field.field_type != FieldType::U32 {
        return error(n, line, "each message must begin with size[4]");
      },
      None => return error(n, line, "each message must begin with size[4]"),
    }

    let (s, name) = name(s);
    let (message_type, message_name, number) = match name {
      Some(result) => result,
      None => return error(n, line, "size tag must be followed by a message name"),
    };

    let (s, tag) = field(s);
    match tag {
      Some(field) => if field.name != "tag" || field.field_type != FieldType::U16 {
        return error(n, line, "each message must have tag[2] as its first field");
      },
      None => return error(n, line, "each message must have tag[2] as its first field"),
    }

    let mut fields: Vec<Field> = vec![];
    let mut s = s;
    while s.len() > 0 {
      let (rest, option) = field(s);
      let field = match option {
        Some(field) => field,
        _ => return error(n, line, &("bad field in message: ".to_owned() + rest)),
      };
      if let Some(ref times) = field.times {
        if let Some(previous) = fields.last() {
          if &previous.name != times || !previous.field_type.integer() {
            return error(n, line, "array field must be preceded by integer times field");
          }
        } else {
          return error(n, line, "array field must be preceded by times field");
        }
      };
      fields.push(field);
      s = rest;
    }

    let message = Message {
      name: message_name.to_owned(),
      full_name: message_type.char().to_string() + message_name,
      message_type: message_type,
      number: number,
      fields: fields
    };
    if names.contains(&message.full_name) {
      return error(n, line, &format!("duplicate message name: {}", message.full_name));
    };
    names.insert(message.full_name.clone());
    if numbers.contains(&message.number) {
      let msg = &format!("duplicate message number: {}:{}", message.full_name, message.number);
      return error(n, line, msg);
    }
    numbers.insert(message.number);
    messages.push(message);
  }

  {
    let mut requests = std::collections::HashMap::<String, &Message>::new();
    let mut responses = std::collections::HashMap::<String, &Message>::new();

    for message in &messages {
      match message.message_type {
        MessageType::Request => requests.insert(message.name.clone(), message),
        MessageType::Response => responses.insert(message.name.clone(), message),
      };
    }

    for name in requests.keys() {
      if !responses.contains_key(name) {
        return error(0, "", &format!("request without corresponding response: {}", name));
      }
    }

    for (name, response) in &responses {
      match requests.get(name) {
        Some(request) => if response.number != request.number + 1 {
          return error(0, "", &format!(
            "Response number not equal to request number + 1: {}\n{}: {}\n{}\n{}\n",
            name, request.full_name, request.number, response.full_name, response.number)
          );
        },
        None => if name != "error" { 
          return error(0, "", &format!("response without corresponding request: {}", name));
        }
      }
    }
  }

  Ok(messages)
}
