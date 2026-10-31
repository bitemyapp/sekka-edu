//! The folder profiles and logs are kept in.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::activity::Event;
use crate::clock;
use crate::profile::{NewProfile, Profile};

/// An app of the catalogue, as it names itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
    /// Its application id (`computer.sekka.Anzan`).
    pub id: String,
    pub name: String,
    /// What it's for, in a few words.
    pub blurb: String,
}

#[derive(Default, Serialize, Deserialize)]
struct ProfilesFile {
    version: u32,
    kids: Vec<Profile>,
}

/// Sekka Kid's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// The folder at `root` (made when first written to).
    pub fn at(root: impl Into<PathBuf>) -> Store {
        Store { root: root.into() }
    }

    /// `$SEKKA_KID_DIR`, else `$XDG_DATA_HOME/sekka-edu`, else
    /// `~/.local/share/sekka-edu`.
    pub fn default_root() -> PathBuf {
        let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        if let Some(dir) = var("SEKKA_KID_DIR") {
            return dir;
        }
        let data = var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
            .unwrap_or_else(std::env::temp_dir);
        data.join("sekka-edu")
    }

    /// The store at [`Store::default_root`].
    pub fn open() -> Store {
        Store::at(Store::default_root())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn profiles_path(&self) -> PathBuf {
        self.root.join("profiles.json")
    }

    /// The kids, in the order they were added.
    pub fn profiles(&self) -> Vec<Profile> {
        read_json::<ProfilesFile>(&self.profiles_path()).map(|f| f.kids).unwrap_or_default()
    }

    pub fn profile(&self, id: &str) -> Option<Profile> {
        self.profiles().into_iter().find(|p| p.id == id)
    }

    /// Change the list under a lock, so apps adding kids at once don't
    /// lose one another's.
    fn edit_profiles<R>(&self, f: impl FnOnce(&mut Vec<Profile>) -> io::Result<R>) -> io::Result<R> {
        fs::create_dir_all(&self.root)?;
        let lock = OpenOptions::new().create(true).truncate(false).write(true).open(self.root.join("profiles.lock"))?;
        // SAFETY: flock on a descriptor we hold open; released when `lock`
        // closes.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut file = read_json::<ProfilesFile>(&self.profiles_path()).unwrap_or_default();
        let r = f(&mut file.kids)?;
        file.version = 1;
        write_json(&self.profiles_path(), &file)?;
        Ok(r)
    }

    /// Add a kid.
    pub fn add(&self, new: NewProfile) -> io::Result<Profile> {
        let name = NewProfile::clean_name(&new.name);
        if name.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "a kid needs a name"));
        }
        let profile = Profile {
            id: new_id(),
            name,
            sounds_like: None,
            animal: new.animal,
            color: new.color,
            grade: new.grade,
            daily_minutes: None,
            created: clock::now_ms(),
        };
        self.edit_profiles(|kids| {
            kids.push(profile.clone());
            Ok(())
        })?;
        fs::create_dir_all(self.kid_dir(&profile.id))?;
        Ok(profile)
    }

    /// Replace the kid with `p`'s id by `p`.
    pub fn update(&self, p: &Profile) -> io::Result<()> {
        self.edit_profiles(|kids| match kids.iter_mut().find(|k| k.id == p.id) {
            Some(k) => {
                *k = Profile {
                    name: NewProfile::clean_name(&p.name),
                    sounds_like: NewProfile::clean_sound(p.sounds_like.as_deref()),
                    ..p.clone()
                };
                Ok(())
            }
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no such kid")),
        })
    }

    /// Remove a kid and everything they played.
    pub fn remove(&self, id: &str) -> io::Result<()> {
        self.edit_profiles(|kids| {
            kids.retain(|k| k.id != id);
            Ok(())
        })?;
        match fs::remove_dir_all(self.kid_dir(id)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// A kid's folder.
    pub fn kid_dir(&self, id: &str) -> PathBuf {
        // Ids are made here, but a hand-edited file shouldn't reach outside.
        let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        self.root.join("kids").join(safe)
    }

    fn log_path(&self, kid: &str) -> PathBuf {
        self.kid_dir(kid).join("activity.jsonl")
    }

    /// Add an event to a kid's log.
    pub fn append(&self, kid: &str, event: &Event) -> io::Result<()> {
        let path = self.log_path(kid);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut line = serde_json::to_vec(event).map_err(io::Error::other)?;
        line.push(b'\n');
        // One write to a file opened for appending: lines from apps writing
        // at once don't interleave.
        OpenOptions::new().create(true).append(true).open(path)?.write_all(&line)
    }

    /// A kid's log, oldest first. Lines that don't read (a newer app's
    /// events, a torn last line) are skipped.
    pub fn events(&self, kid: &str) -> Vec<Event> {
        let mut text = String::new();
        if File::open(self.log_path(kid)).and_then(|mut f| f.read_to_string(&mut text)).is_err() {
            return Vec::new();
        }
        text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
    }

    fn progress_path(&self, kid: &str, app: &str) -> PathBuf {
        let safe: String = app.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').collect();
        self.kid_dir(kid).join("apps").join(format!("{safe}.json"))
    }

    /// An app's own record of a kid (where they've got to), or its default.
    pub fn progress<T: DeserializeOwned + Default>(&self, kid: &str, app: &str) -> T {
        read_json(&self.progress_path(kid, app)).unwrap_or_default()
    }

    pub fn save_progress<T: Serialize>(&self, kid: &str, app: &str, value: &T) -> io::Result<()> {
        write_json(&self.progress_path(kid, app), value)
    }

    /// Say an app is part of the catalogue, so reports can name it.
    pub fn register_app(&self, entry: &AppEntry) -> io::Result<()> {
        let path = self.root.join("catalogue").join(format!("{}.json", entry.id));
        if read_json::<AppEntry>(&path).as_ref() == Some(entry) {
            return Ok(());
        }
        write_json(&path, entry)
    }

    /// The apps that have run, by name.
    pub fn apps(&self) -> Vec<AppEntry> {
        let mut apps: Vec<AppEntry> = fs::read_dir(self.root.join("catalogue"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| read_json(&e.path()))
            .collect();
        apps.sort_by(|a, b| a.name.cmp(&b.name));
        apps
    }

    /// An app's name, or its id if it never said.
    pub fn app_name(&self, id: &str) -> String {
        read_json::<AppEntry>(&self.root.join("catalogue").join(format!("{id}.json")))
            .map(|a| a.name)
            .unwrap_or_else(|| id.rsplit('.').next().unwrap_or(id).to_owned())
    }

    /// Milliseconds a kid has played today, across apps, from the log.
    pub fn played_today(&self, kid: &str, now: i64) -> i64 {
        let (from, to) = (clock::day_start(now), clock::next_day_start(now));
        self.events(kid)
            .iter()
            .filter_map(|e| match e {
                Event::Active { from: a, to: b, .. } => Some(((*a).max(from), (*b).min(to))),
                _ => None,
            })
            .map(|(a, b)| (b - a).max(0))
            .sum()
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("{}: {e}", path.display());
            None
        }
    }
}

/// Write a whole file by renaming a finished copy over it.
fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::process::id()
    ));
    let mut text = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    text.push(b'\n');
    {
        let mut f = File::create(&tmp)?;
        f.write_all(&text)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// Sixteen hex digits from the kernel's random numbers.
fn new_id() -> String {
    let mut bytes = [0u8; 8];
    if File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes)).is_err() {
        let seed = clock::now_ms() as u64 ^ (u64::from(std::process::id()) << 40);
        bytes = seed.to_le_bytes();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
