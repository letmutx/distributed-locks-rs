use std::borrow::Cow;
use memcache::{Client, Connectable, MemcacheError};
use std::time::Duration;

const MEMCACHE_DEFAULT_URL: &'static str = "memcache://localhost:11211";

pub struct GhettoLock<'a> {
    retries: usize,
    name: Cow<'a, String>,
    /// Memcache client instance.
    memcache: Client,
    /// Expiry in seconds.
    expiry: u32,
    /// The value set in the key to identify the current owner of the key.
    owner: String,
}

// TODO:
// * auto-renewal option
// * check cluster support
// * failure cases
// * tests
// * implement drop for guard to unlock the key
// * what should be clone, send semantics?
// * should ghetto_lock have internal mutability?

pub struct Guard;

pub type LockResult = Result<Guard, Error>;

impl<'a> GhettoLock<'a> {
    pub fn try_lock(&mut self) -> LockResult {
        for _ in 0..self.retries {
            match self.memcache.add(&self.name, &*self.owner, self.expiry) {
                Ok(()) => {
                    // TODO: check time validity
                    // (current time - time before the above call < expiry time)
                    return Ok(Guard);
                }
                _ => {
                    // TODO: handle memcache errors
                    // TODO: add sleep( nearly equal to expiry time) before retrying 
                }
            }
        }
        Err(Error::RetriesExceeded)
    }
}


pub enum Error {
    Memcache(MemcacheError),
    RetriesExceeded
}

impl From<MemcacheError> for Error {
    fn from(error: MemcacheError) -> Self {
        Self::Memcache(error)
    }
}

pub struct LockOptions<'a, C> {
    name: Cow<'a, String>,
    connectable: C,
    expiry: Option<u32>,
    owner: String,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    retries: usize,
}

impl<'a> LockOptions<'a, &str> {
    pub fn new(name: Cow<'a, String>, owner: String) -> Self {
        Self {
            name,
            connectable: MEMCACHE_DEFAULT_URL,
            expiry: None,
            owner: owner,
            read_timeout: None,
            write_timeout: None,
            retries: 5,
        }
    }
}

impl<'a, C: Connectable> LockOptions<'a, C> {
    pub fn with_expiry(mut self, expiry: u32) -> Self {
        self.expiry = Some(expiry);
        self
    }

    pub fn with_connectable<K: Connectable>(self, connectable: K) -> LockOptions<'a, K> {
        LockOptions {
            connectable: connectable,
            name: self.name,
            expiry: self.expiry,
            owner: self.owner,
            read_timeout: self.read_timeout,
            write_timeout: self.write_timeout,
            retries: self.retries
        }
    }

    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = Some(timeout);
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    pub fn build(self) -> Result<GhettoLock<'a>, Error> {
        let mut memcache = Client::connect(self.connectable)?;
        memcache.set_read_timeout(self.read_timeout)?;
        memcache.set_write_timeout(self.write_timeout)?;
        Ok(GhettoLock {
            retries: self.retries,
            name: self.name,
            memcache,
            owner: self.owner,
            expiry: self.expiry.unwrap_or(10),
        })
    }
}