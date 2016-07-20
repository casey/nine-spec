#![cfg(test)]

extern crate brev;

use super::*;

#[test]
fn round_trip() {
  let description = brev::slurp("9p2000.txt");
  let messages = match parse(&description) {
    Ok(messages) => messages,
    Err(error) => brev::die(error),
  };

  let mut stripped = String::new();

  for (_, line) in strip(&description) {
    stripped.push_str(line);
    stripped.push_str("\n");
  }

  let mut round_tripped = String::new();
  for message in messages {
    round_tripped.push_str(&format!("{}", message));
    round_tripped.push_str("\n");
  }

  if round_tripped != stripped {
    println!("round tripped description does not match stripped description:");
    println!("stripped:\n{}", stripped);
    println!("round tripped:\n{}", round_tripped);
    assert!(false);
  }
}

#[test]
fn duplicate_numbers() {
  let description = r"
    size[4] Tversion:100 tag[2]
    size[4] Rversion:101 tag[2]

    size[4] Tauth:100 tag[2]
    size[4] Rauth:101 tag[2]
  ";
  
  if let Ok(_) = parse(description) {
    println!("successfully parsed message with duplicate number: {}", description);
    assert!(false);
  }
}

#[test]
fn duplicate_names() {
  let description = r"
    size[4] Thello:77 tag[2]
    size[4] Rhello:78 tag[2]

    size[4] Thello:33 tag[2]
    size[4] Rhello:34 tag[2]
  ";
  
  if let Ok(_) = parse(description) {
    println!("successfully parsed message with duplicate name: {}", description);
    assert!(false);
  }
}

#[test]
fn mismatched_numbers() {
  let description = r"
    size[4] Thello:77 tag[2]
    size[4] Rhello:76 tag[2]
  ";
  
  if let Ok(_) = parse(description) {
    println!("successfully parsed message with mismatched numbers: {}", description);
    assert!(false);
  }
}

#[test]
fn missing_request() {
  let description = r"
    size[4] Rhello:1 tag[2]
  ";
  
  if let Ok(_) = parse(description) {
    println!("successfully parsed message with missing request: {}", description);
    assert!(false);
  }
}

#[test]
fn missing_response() {
  let description = r"
    size[4] Thello:0 tag[2]
  ";
  
  if let Ok(_) = parse(description) {
    println!("successfully parsed message with missing request: {}", description);
    assert!(false);
  }
}
