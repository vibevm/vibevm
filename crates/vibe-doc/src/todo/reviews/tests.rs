//! Reading the record of who read what, and when.

use std::path::Path;

use super::*;
use crate::pages;

const TODAY: &str = "2026-09-12";

fn today() -> NaiveDate {
    date(TODAY).expect("a date")
}

fn package(dir: &Path, reviews: Option<&str>) {
    let pages = dir.join("vibevm/vibespecs/model");
    std::fs::create_dir_all(&pages).expect("the spec root");
    for name in ["boot-lane.xml", "packages.xml"] {
        std::fs::write(
            pages.join(name),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Page</title>\n\
             </spec>\n",
        )
        .expect("a page");
    }
    if let Some(text) = reviews {
        let maintenance = dir.join("maintenance");
        std::fs::create_dir_all(&maintenance).expect("the maintenance directory");
        std::fs::write(maintenance.join("reviews.toml"), text).expect("the record");
    }
}

#[test]
fn a_package_with_no_record_has_no_readings() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path(), None);
    assert!(read(tmp.path()).expect("a reading").is_none());
}

#[test]
fn a_recorded_reading_comes_back_with_its_reader() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(
        tmp.path(),
        Some(
            "schema = 1\nreconciled = \"2026-06-14\"\n\n\
             [[page]]\npath = \"model/boot-lane.xml\"\nread = \"2026-09-01\"\nby = \"owner\"\n",
        ),
    );
    let reviews = read(tmp.path()).expect("a reading").expect("a record");
    let set = pages::read_package(tmp.path()).expect("the pages");
    let ages = ages(&reviews, &set, None, tmp.path());
    assert_eq!(ages.len(), 2, "every page of the package has a standing");
    assert_eq!(ages[0].page, "model/boot-lane.xml");
    assert_eq!(ages[0].by, "owner");
    assert_eq!(ages[0].days(today()), Some(11));
    assert_eq!(
        ages[1].page, "model/packages.xml",
        "the rota comes first and the unread pages after it"
    );
    assert_eq!(ages[1].days(today()), None);
    assert_eq!(days_since_reconcile(&reviews, today()), Some(90));
}

#[test]
fn a_record_that_does_not_parse_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path(), Some("schema = \"one\"\n"));
    let error = read(tmp.path()).expect_err("a refusal").to_string();
    assert!(error.contains("does not parse"), "{error}");
    assert!(error.contains("OBS-MAINTENANCE-TOOLS"), "{error}");
}

#[test]
fn a_date_nobody_can_read_makes_a_page_unread_and_not_young() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(
        tmp.path(),
        Some(
            "schema = 1\n\n[[page]]\npath = \"model/boot-lane.xml\"\n\
             read = \"the other day\"\nby = \"owner\"\n",
        ),
    );
    let reviews = read(tmp.path()).expect("a reading").expect("a record");
    let set = pages::read_package(tmp.path()).expect("the pages");
    let ages = ages(&reviews, &set, None, tmp.path());
    assert_eq!(
        ages[0].days(today()),
        None,
        "an unreadable date must not become a young page — the safe direction is \
         «owed a reading»"
    );
}

#[test]
fn with_no_checkout_the_rule_of_five_is_not_asked() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(
        tmp.path(),
        Some(
            "schema = 1\n\n[[page]]\npath = \"model/boot-lane.xml\"\n\
             read = \"2026-09-01\"\nby = \"owner\"\n",
        ),
    );
    let reviews = read(tmp.path()).expect("a reading").expect("a record");
    let set = pages::read_package(tmp.path()).expect("the pages");
    assert_eq!(ages(&reviews, &set, None, tmp.path())[0].edits, None);
}
