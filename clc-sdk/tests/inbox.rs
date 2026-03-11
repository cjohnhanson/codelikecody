use clc_sdk::inbox::{FolderInbox, Inbox};
use std::fs;
use tempfile::TempDir;

#[test]
fn empty_inbox_returns_no_items() {
    let dir = TempDir::new().unwrap();
    let mut inbox = FolderInbox::new(dir.path());
    let items = inbox.poll().unwrap();
    assert!(items.is_empty());
}

#[test]
fn single_file_returned_as_item() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("msg.txt"), "hello").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    let items = inbox.poll().unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content(), "hello");
}

#[test]
fn multiple_files_all_returned() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();
    fs::write(dir.path().join("c.txt"), "ccc").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    let items = inbox.poll().unwrap();

    assert_eq!(items.len(), 3);
}

#[test]
fn files_not_returned_on_second_poll() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("msg.txt"), "hello").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    let first = inbox.poll().unwrap();
    assert_eq!(first.len(), 1);

    let second = inbox.poll().unwrap();
    assert!(second.is_empty());
}

#[test]
fn processed_files_no_longer_in_inbox_root() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("msg.txt"), "hello").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    inbox.poll().unwrap();

    assert!(!dir.path().join("msg.txt").exists());
}

#[test]
fn new_files_picked_up_on_subsequent_poll() {
    let dir = TempDir::new().unwrap();
    let mut inbox = FolderInbox::new(dir.path());

    fs::write(dir.path().join("msg1.txt"), "first").unwrap();
    let first = inbox.poll().unwrap();
    assert_eq!(first.len(), 1);

    fs::write(dir.path().join("msg2.txt"), "second").unwrap();
    let second = inbox.poll().unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].content(), "second");
}

#[test]
fn item_source_reflects_filename() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("my-message.txt"), "content").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    let items = inbox.poll().unwrap();

    assert_eq!(items[0].source(), "my-message.txt");
}

#[test]
fn subdirectories_are_ignored() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("subdir").join("nested.txt"), "nested").unwrap();
    fs::write(dir.path().join("msg.txt"), "top-level").unwrap();

    let mut inbox = FolderInbox::new(dir.path());
    let items = inbox.poll().unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content(), "top-level");
}
