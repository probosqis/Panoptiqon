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

use std::sync::atomic::AtomicPtr;
use std::sync::mpsc::Sender;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::JoinHandle;
use crate::db::save_task::SaveTask;

static SINGLETON: LazyLock<Arc<DbScheduler>>
   = LazyLock::new(|| Arc::new(DbScheduler::new()));

pub(crate) struct DbScheduler {
   worker_thread: Mutex<WorkerThread>,
   worker_thread_message_sender: AtomicPtr<Sender<WorkerThreadMessage>>
}

impl DbScheduler {
   pub(crate) fn singleton() -> Arc<Self> {
      Arc::clone(&SINGLETON)
   }

   pub(crate) const fn new() -> Self {
      use std::ptr;

      Self {
         worker_thread: Mutex::new(WorkerThread::NotStarted),
         worker_thread_message_sender: AtomicPtr::new(ptr::null_mut())
      }
   }

   /// WorkerThreadにメッセージを送信するためのSenderを取得する。
   ///
   /// WorkerThreadが起動されていない場合、**WorkerThreadを起動して**
   /// そのSenderを返却する。
   fn message_sender(&self) -> &Sender<WorkerThreadMessage> {
      use std::sync::atomic::Ordering;

      let sender = unsafe {
         self.worker_thread_message_sender.load(Ordering::Relaxed).as_ref()
      };

      // worker_thread_message_senderが非nullの場合。
      // ロックなしで返却可能
      if let Some(sender) = sender { return sender; }

      // worker_thread_message_senderがnullの場合。
      // ロックを取得
      let mut lock = self.worker_thread.lock().unwrap();

      let message_sender = match *lock {
         WorkerThread::Running { ref mut message, .. } => {
            // WorkerThreadが起動済み。
            // worker_thread_message_senderがnullでも
            // 同時に別スレッドがWorkerThreadを起動していた場合は
            // ロックが取れたあとにWorkerThreadが起動されていることはありうる
            message
         }
         WorkerThread::NotStarted => {
            // WorkerThreadが起動されていない。起動する
            lock.start();

            // worker_thread_message_senderに起動したインスタンスを格納
            let WorkerThread::Running { ref mut message, .. } = *lock else {
               panic!();
            };

            self.worker_thread_message_sender
               .store(message as *mut _, Ordering::Relaxed);

            message
         }
      };

      let message_sender_ptr = message_sender as *mut _;

      // ロックを解除（タイミングを明確にするため明示）
      drop(lock);

      unsafe { &*message_sender_ptr }
   }

   pub(crate) fn push(&self, task: SaveTask) {
      let sender = self.message_sender();
      sender.send(WorkerThreadMessage::SaveTask(task)).unwrap();
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn stop(&self) -> Vec<SaveTask> {
      use std::ptr;
      use std::sync::atomic::Ordering;

      let mut lock = self.worker_thread.lock().unwrap();
      self.worker_thread_message_sender
         .store(ptr::null_mut(), Ordering::Relaxed);
      lock.stop()
   }
}

impl Drop for DbScheduler {
   fn drop(&mut self) {
      use std::ptr;
      use std::sync::atomic::Ordering;

      if let Ok(mut worker_thread) = self.worker_thread.lock() {
         self.worker_thread_message_sender
            .store(ptr::null_mut(), Ordering::Relaxed);
         worker_thread.stop();
      }
   }
}

enum WorkerThread {
   NotStarted,

   Running {
      #[cfg(not(any(test, feature = "jni-test")))]
      handle: JoinHandle<anyhow::Result<()>>,
      #[cfg(any(test, feature = "jni-test"))]
      handle: JoinHandle<Vec<SaveTask>>,
      message: Sender<WorkerThreadMessage>
   }
}

enum WorkerThreadMessage {
   Stop,
   SaveTask(SaveTask),
}

impl WorkerThread {
   fn start(&mut self) {
      use std::sync::mpsc;
      use std::thread;

      if let WorkerThread::Running { .. } = self { return; }

      let (tx, rx) = mpsc::channel();

      #[cfg(not(any(test, feature = "jni-test")))]
      let handle = thread::spawn(move || {
         use crate::db::saver::Saver;

         let mut errs = Vec::new();

         loop {
            let Ok(message) = rx.recv() else { break; };

            match message {
               WorkerThreadMessage::Stop => break,
               WorkerThreadMessage::SaveTask(task) => {
                  let r = Saver::save(task);
                  if r.is_err() {
                     errs.push(r);
                  }
               }
            }
         }

         errs.into_iter().collect()
      });

      #[cfg(any(test, feature = "jni-test"))]
      let handle = thread::spawn(move || {
         let mut received_tasks = Vec::new();

         loop {
            let Ok(message) = rx.recv() else { break received_tasks; };

            match message {
               WorkerThreadMessage::Stop => break received_tasks,
               WorkerThreadMessage::SaveTask(task) => {
                  received_tasks.push(task);
               }
            }
         }
      });

      *self = Self::Running {
         handle,
         message: tx
      };
   }

   #[cfg(not(any(test, feature = "jni-test")))]
   fn stop(&mut self) {
      use std::mem;

      let WorkerThread::Running { handle, message }
         = mem::replace(self, Self::NotStarted) else { return; };

      message.send(WorkerThreadMessage::Stop).unwrap();
      handle.join().unwrap();
   }

   #[cfg(any(test, feature = "jni-test"))]
   fn stop(&mut self) -> Vec<SaveTask> {
      use std::mem;

      let WorkerThread::Running { handle, message }
         = mem::replace(self, Self::NotStarted) else { return vec![]; }; 

      message.send(WorkerThreadMessage::Stop).unwrap();
      handle.join().unwrap()
   }
}

#[cfg(test)]
mod test {
   use std::path::PathBuf;
   use crate::db::savable::Savable;
   use crate::db::save_task::SaveTask;
   use crate::db::saver;
   use super::{DbScheduler, WorkerThread};

   struct SavableImpl;
   impl Savable for SavableImpl {
      fn file_path(&self) -> PathBuf {
         PathBuf::from("DbSchedulerTest/SavableImpl")
      }

      fn serialize(
         &self,
         _serializer: saver::Serializer
      ) -> anyhow::Result<
         <saver::Serializer as serde::Serializer>::Ok,
         <saver::Serializer as serde::Serializer>::Error
      > {
         unimplemented!();
      }
   }

   #[allow(non_snake_case)]
   #[test]
   fn push_startWorkerThread() {
      use std::sync::Arc;
      use std::sync::atomic::Ordering;

      let db_scheduler = DbScheduler::new();

      assert!(
         matches!(
            *db_scheduler.worker_thread.lock().unwrap(),
            WorkerThread::NotStarted
         )
      );
      assert!(
         db_scheduler.worker_thread_message_sender.load(Ordering::Relaxed)
            .is_null()
      );

      db_scheduler.push(
         SaveTask::new(Arc::new(SavableImpl))
      );

      assert!(
         matches!(
            *db_scheduler.worker_thread.lock().unwrap(),
            WorkerThread::Running { .. }
         )
      );
      assert!(
         !db_scheduler.worker_thread_message_sender.load(Ordering::Relaxed)
            .is_null()
      );
   }
}
