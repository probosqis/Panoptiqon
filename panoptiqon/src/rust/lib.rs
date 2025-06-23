/*
 * Copyright 2024-2025 wcaokaze
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#![allow(incomplete_features)]
#![feature(
   get_mut_unchecked, mapped_lock_guards, never_type, ptr_metadata,
   specialization
)]

use crate::db::scheduler::DbScheduler;

pub mod cache;
pub mod repository;

pub(crate) mod db;
mod pool;
mod unique_cache;

#[cfg(feature = "jvm")]
pub mod convert_jvm;
#[cfg(feature = "jvm")]
pub mod jvm_type;
#[cfg(feature = "jvm")]
pub mod jvm_types;

pub struct Panoptiqon {
   db_scheduler: DbScheduler
}

impl Panoptiqon {
   pub const fn new() -> Self {
      Panoptiqon {
         db_scheduler: DbScheduler::new()
      }
   }
}
