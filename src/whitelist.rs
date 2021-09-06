use std::{collections::BTreeSet, ffi::OsString, path::Path};

lazy_static! {
	static ref WHITELIST: BTreeSet<OsString> = [
		"txt", "dat", "json", "xml", "csv", "jpg",
		"jpeg", "png", "vtf", "vmt", "mp3", "wav",
		"ogg",
	]
	.iter()
	.map(|ext| OsString::from(ext))
	.collect::<BTreeSet<OsString>>();
}

pub fn check<P: AsRef<Path>>(path: P) -> bool {
	if let Some(ext) = path.as_ref().extension() {
		WHITELIST.contains(ext)
	} else {
		false
	}
}
