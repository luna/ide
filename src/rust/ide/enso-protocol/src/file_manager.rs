//! Client library for the JSON-RPC-based File Manager service.

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![warn(unsafe_code)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]

use crate::prelude::*;

use crate::common::UTCDateTime;

use json_rpc::api::Result;
use json_rpc::Handler;
use json_rpc::make_arg;
use json_rpc::make_param_map;
use json_rpc::make_rpc_methods;
use futures::Stream;
use serde::Serialize;
use serde::Deserialize;
use std::future::Future;
use uuid::Uuid;



// =============
// === Event ===
// =============

/// Event emitted by the File Manager `Client`.
pub type Event = json_rpc::handler::Event<Notification>;



// ============
// === Path ===
// ============

/// Path to a file.
#[derive(Clone,Debug,Display,Eq,Hash,PartialEq,PartialOrd,Ord)]
#[derive(Serialize, Deserialize)]
#[derive(Shrinkwrap)]
pub struct Path(pub String);

impl Path {
    /// Wraps a `String`-like entity into a new `Path`.
    pub fn new(s:impl Str) -> Path {
        Path(s.into())
    }
}



// ====================
// === Notification ===
// ====================

/// Notification generated by the File Manager.
#[derive(Clone,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(tag="method", content="params")]
pub enum Notification {
    /// Filesystem event occurred for a watched path.
    #[serde(rename = "filesystemEvent")]
    FilesystemEvent(FilesystemEvent),
}



// =======================
// === FilesystemEvent ===
// =======================

/// Filesystem event notification, generated by an active file watch.
#[derive(Clone,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct FilesystemEvent {
    /// Path of the file that the event is about.
    pub path : Path,
    /// What kind of event is it.
    pub kind : FilesystemEventKind
}

/// Describes kind of filesystem event (was the file created or deleted, etc.)
#[derive(Clone,Copy,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
pub enum FilesystemEventKind {
    /// A new file under path was created.
    Created,
    /// Existing file under path was deleted.
    Deleted,
    /// File under path was modified.
    Modified,
    /// An overflow occurred and some events were lost,
    Overflow
}



// ==================
// === Attributes ===
// ==================

/// Attributes of the file in the filesystem.
#[derive(Clone,Copy,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes{
    /// When the file was created.
    pub creation_time      : UTCDateTime,
    /// When the file was last accessed.
    pub last_access_time   : UTCDateTime,
    /// When the file was last modified.
    pub last_modified_time : UTCDateTime,
    /// What kind of file is this.
    pub file_kind          : FileKind,
    /// Size of the file in bytes.
    /// (size of files not being `RegularFile`s is unspecified).
    pub byte_size          : u64
}

/// What kind of file (regular, directory, symlink) is this.
#[derive(Clone,Copy,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
pub enum FileKind {
    /// File being a directory.
    Directory,
    /// File being a symbolic link.
    SymbolicLink,
    /// File being a regular file with opaque content.
    RegularFile,
    /// File being none of the above, e.g. a physical device or a pipe.
    Other
}

make_rpc_methods! {
/// An interface containing all the available file management operations.
trait API {
    /// Copies a specified directory to another location.
    #[MethodInput=CopyDirectoryInput,rpc_name="file/copy",result=copy_directory_result,set_result=set_copy_directory_result]
    fn copy_directory(&self, from:Path, to:Path) -> ();

    /// Copies a specified file to another location.
    #[MethodInput=CopyFileInput,rpc_name="file/copy",result=copy_file_result,set_result=set_copy_file_result]
    fn copy_file(&self, from:Path, to:Path) -> ();

    /// Deletes the specified file.
    #[MethodInput=DeleteFileInput,rpc_name="file/delete",result=delete_file_result,
    set_result=set_delete_file_result]
    fn delete_file(&self, path:Path) -> ();

    /// Check if file exists.
    #[MethodInput=ExistsInput,rpc_name="file/exists",result=exists_result,
    set_result=set_exists_result]
    fn exists(&self, path:Path) -> bool;

    /// List all file-system objects in the specified path.
    #[MethodInput=ListInput,rpc_name="file/list",result=list_result,set_result=set_list_result]
    fn list(&self, path:Path) -> Vec<Path>;

    /// Moves directory to another location.
    #[MethodInput=MoveDirectoryInput,rpc_name="file/move",result=move_directory_result,
    set_result=set_move_directory_result]
    fn move_directory(&self, from:Path, to:Path) -> ();

    /// Moves file to another location.
    #[MethodInput=MoveFileInput,rpc_name="file/move",result=move_file_result,
    set_result=set_move_file_result]
    fn move_file(&self, from:Path, to:Path) -> ();

    /// Reads file's content as a String.
    #[MethodInput=ReadInput,rpc_name="file/read",result=read_result,set_result=set_read_result]
    fn read(&self, path:Path) -> String;

    /// Gets file's status.
    #[MethodInput=StatusInput,rpc_name="file/status",result=status_result,set_result=set_status_result]
    fn status(&self, path:Path) -> Attributes;

    /// Creates a file in the specified path.
    #[MethodInput=TouchInput,rpc_name="file/touch",result=touch_result,set_result=set_touch_result]
    fn touch(&self, path:Path) -> ();

    /// Writes String contents to a file in the specified path.
    #[MethodInput=WriteInput,rpc_name="file/write",result=write_result,set_result=set_write_result]
    fn write(&self, path:Path, contents:String) -> ();

    /// Watches the specified path.
    #[MethodInput=CreateWatchInput,rpc_name="file/createWatch",result=create_watch_result,set_result=set_create_watch_result]
    fn create_watch(&self, path:Path) -> Uuid;

    /// Delete the specified watcher.
    #[MethodInput=DeleteWatchInput,rpc_name="file/deleteWatch",result=delete_watch_result,
    set_result=set_delete_watch_result]
    fn delete_watch(&self, watch_id:Uuid) -> ();
}
}



// =============
// === Tests ===
// =============

#[cfg(test)]
mod tests {
    use super::*;
    use super::FileKind::RegularFile;

    use json_rpc::messages::Message;
    use json_rpc::messages::RequestMessage;
    use json_rpc::test_util::transport::mock::MockTransport;
    use serde_json::json;
    use serde_json::Value;
    use std::future::Future;
    use utils::test::poll_future_output;
    use utils::test::poll_stream_output;
    use futures::task::LocalSpawnExt;

    struct Fixture {
        transport : MockTransport,
        client    : Client,
        executor  : futures::executor::LocalPool,
    }

    fn setup_fm() -> Fixture {
        let transport = MockTransport::new();
        let client    = Client::new(transport.clone());
        let executor  = futures::executor::LocalPool::new();
        executor.spawner().spawn_local(client.runner()).unwrap();
        Fixture {transport,client,executor}
    }

    #[test]
    fn test_notification() {
        let mut fixture = setup_fm();
        let mut events  = Box::pin(fixture.client.events());
        assert!(poll_stream_output(&mut events).is_none());

        let expected_notification = FilesystemEvent {
            path : Path::new("./Main.luna"),
            kind : FilesystemEventKind::Modified,
        };
        let notification_text = r#"{
            "jsonrpc": "2.0",
            "method": "filesystemEvent",
            "params": {"path" : "./Main.luna", "kind" : "Modified"}
        }"#;
        fixture.transport.mock_peer_message_text(notification_text);
        assert!(poll_stream_output(&mut events).is_none());

        fixture.executor.run_until_stalled();

        let event = poll_stream_output(&mut events);
        if let Some(Event::Notification(n)) = event {
            assert_eq!(n, Notification::FilesystemEvent(expected_notification));
        } else {
            panic!("expected notification event");
        }
    }

    /// Tests making a request using file manager:
    /// * creates FM client and uses `make_request` to make a request
    /// * checks that request is made for `expected_method`
    /// * checks that request input is `expected_input`
    /// * mocks receiving a response from server with `result`
    /// * checks that FM-returned Future yields `expected_output`
    fn test_request<Fun, Fut, T>
    ( make_request:Fun
    , expected_method:&str
    , expected_input:Value
    , result:Value
    , expected_output:T )
    where Fun : FnOnce(&mut Client) -> Fut,
          Fut : Future<Output = Result<T>>,
          T   : Debug + PartialEq {
        let mut fixture = setup_fm();
        let mut fut     = Box::pin(make_request(&mut fixture.client));

        let request = fixture.transport.expect_message::<RequestMessage<Value>>();
        assert_eq!(request.method, expected_method);
        assert_eq!(request.params, expected_input);

        let response = Message::new_success(request.id, result);
        fixture.transport.mock_peer_message(response);
        fixture.executor.run_until_stalled();
        let output = poll_future_output(&mut fut).unwrap().unwrap();
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_requests() {
        let main                = Path::new("./Main.luna");
        let target              = Path::new("./Target.luna");
        let path_main           = json!({"path" : "./Main.luna"});
        let from_main_to_target = json!({
            "from" : "./Main.luna",
            "to"   : "./Target.luna"
        });
        let true_json = json!(true);
        let unit_json = json!(null);

        test_request(
            |client| client.copy_directory(main.clone(), target.clone()),
            "file/copy",
            from_main_to_target.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.copy_file(main.clone(), target.clone()),
            "file/copy",
            from_main_to_target.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.delete_file(main.clone()),
            "file/delete",
            path_main.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.exists(main.clone()),
            "file/exists",
            path_main.clone(),
            true_json,
            true);

        let list_response_json  = json!([          "Bar.luna",           "Foo.luna" ]);
        let list_response_value = vec!  [Path::new("Bar.luna"),Path::new("Foo.luna")];
        test_request(
            |client| client.list(main.clone()),
            "file/list",
            path_main.clone(),
            list_response_json,
            list_response_value);
        test_request(
            |client| client.move_directory(main.clone(), target.clone()),
            "file/move",
            from_main_to_target.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.move_file(main.clone(), target.clone()),
            "file/move",
            from_main_to_target.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.read(main.clone()),
            "file/read",
            path_main.clone(),
            json!("Hello world!"),
            "Hello world!".into());

        let parse_rfc3339 = |s| {
            chrono::DateTime::parse_from_rfc3339(s).unwrap()
        };
        let expected_attributes = Attributes {
            creation_time      : parse_rfc3339("2020-01-07T21:25:26Z"),
            last_access_time   : parse_rfc3339("2020-01-21T22:16:51.123994500+00:00"),
            last_modified_time : parse_rfc3339("2020-01-07T21:25:26Z"),
            file_kind          : RegularFile,
            byte_size          : 125125,
        };
        let sample_attributes_json = json!({
            "creationTime"      : "2020-01-07T21:25:26Z",
            "lastAccessTime"    : "2020-01-21T22:16:51.123994500+00:00",
            "lastModifiedTime"  : "2020-01-07T21:25:26Z",
            "fileKind"          : "RegularFile",
            "byteSize"          : 125125
        });
        test_request(
            |client| client.status(main.clone()),
            "file/status",
            path_main.clone(),
            sample_attributes_json,
            expected_attributes);
        test_request(
            |client| client.touch(main.clone()),
            "file/touch",
            path_main.clone(),
            unit_json.clone(),
            ());
        test_request(
            |client| client.write(main.clone(), "Hello world!".into()),
            "file/write",
            json!({"path" : "./Main.luna", "contents" : "Hello world!"}),
            unit_json.clone(),
            ());

        let uuid_value = uuid::Uuid::parse_str("02723954-fbb0-4641-af53-cec0883f260a").unwrap();
        let uuid_json  = json!("02723954-fbb0-4641-af53-cec0883f260a");
        test_request(
            |client| client.create_watch(main.clone()),
            "file/createWatch",
            path_main.clone(),
            uuid_json.clone(),
            uuid_value);
        let watch_id   = json!({
            "watchId" : "02723954-fbb0-4641-af53-cec0883f260a"
        });
        test_request(
            |client| client.delete_watch(uuid_value.clone()),
            "file/deleteWatch",
            watch_id.clone(),
            unit_json.clone(),
            ());
    }
}
