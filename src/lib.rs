#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate quick_error;
extern crate redis;

use self::errors::*;
use self::scripts::*;
use self::redlock::*;

mod errors;
mod scripts;
mod redlock;
