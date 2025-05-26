/*
 * Copyright 2025 wcaokaze
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

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::thread::JoinHandle;
use crate::db::save_task::SaveTask;

#[cfg(not(any(test, feature = "jni-test")))]
static SINGLETON: DbScheduler = DbScheduler::new();

#[cfg(any(test, feature = "jni-test"))]
thread_local! {
   static SINGLETON: DbScheduler = DbScheduler::new();
}

pub(crate) struct DbScheduler {
   tasks: Mutex<VecDeque<SaveTask>>,
   is_running: AtomicBool,
   worker_thread: Mutex<Option<WorkerThread>>
}

impl DbScheduler {
   const fn new() -> Self {
      Self {
         tasks: Mutex::new(VecDeque::new()),
         is_running: AtomicBool::new(false),
         worker_thread: Mutex::new(None)
      }
   }

   fn with_singleton<R>(f: impl FnOnce(&DbScheduler) -> R) -> R {
      #[cfg(not(any(test, feature = "jni-test")))]
      {
         f(&SINGLETON)
      }

      #[cfg(any(test, feature = "jni-test"))]
      {
         SINGLETON.with(f)
      }
   }

   fn start_worker_thread(&self) {
      use std::sync::atomic::Ordering;

      if self.is_running
         .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
         .is_ok()
      {
         let mut lock = self.worker_thread.lock().unwrap();
         if let Some(_) = *lock { return; }

         *lock = Some(WorkerThread::start());
      }
   }

   pub(crate) fn push(task: SaveTask) {
      Self::with_singleton(|singleton| {
         singleton.start_worker_thread();
         singleton.tasks.lock().unwrap().push_front(task);
      });
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn clear_all_tasks() {
      Self::with_singleton(|singleton| {
         singleton.tasks.lock().unwrap().clear();
      });
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn tasks() -> VecDeque<SaveTask> {
      Self::with_singleton(|singleton| {
         singleton.tasks.lock().unwrap().clone()
      })
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn kill_worker_thread() {
      use std::sync::atomic::Ordering;

      Self::with_singleton(|singleton| {
         if let Some(worker_thread) = singleton.worker_thread.lock().unwrap().take() {
            worker_thread.stop();
            singleton.is_running.store(false, Ordering::Relaxed);
         }
      });
   }
}

struct WorkerThread {
   handle: JoinHandle<()>,
   stop_request: Sender<()>
}

impl WorkerThread {
   fn start() -> Self {
      use std::sync::mpsc;
      use std::thread;

      let (tx, rx) = mpsc::channel();

      let handle = thread::spawn(move || loop {
         let Ok(()) = rx.recv() else { break; };

      });

      Self {
         handle,
         stop_request: tx
      }
   }

   fn stop(self) {
      self.stop_request.send(()).unwrap();
      self.handle.join().unwrap();
   }
}
