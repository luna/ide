//! Module Controller.
//!
//! The module controller keeps cached module state (module state is AST+Metadata or equivalent),
//! and uses it for synchronizing state for text and graph representations. It provides method
//! for registering text and graph changes. If for example text represntation will be changed, there
//! will be notifications for both text change and graph change.

use crate::prelude::*;

use crate::controller::FallibleResult;
use crate::controller::notification;
use crate::double_representation::apply_code_change_to_id_map;
use crate::executor::global::spawn;

use ast;
use ast::Ast;
use ast::HasRepr;
use ast::IdMap;
use data::text::Index;
use data::text::Size;
use data::text::Span;
use data::text::TextChangedInfo;
use file_manager_client as fmc;
use flo_stream::MessagePublisher;
use flo_stream::Subscriber;
use futures::future;
use json_rpc::error::RpcError;
use parser::api::IsParser;
use parser::Parser;
use shapely::shared;


// =======================
// === Module Location ===
// =======================

/// Structure uniquely identifying module location in the project.
/// Mappable to filesystem path.
#[derive(Clone,Debug,Display,Eq,Hash,PartialEq)]
pub struct Location(pub String);

impl Location {
    /// Get the module location from filesystem path. Returns None if path does not lead to
    /// module file.
    pub fn from_path(path:&fmc::Path) -> Option<Self> {
        // TODO [ao] See function `to_path`
        let fmc::Path(path_str) = path;
        let suffix = format!(".{}", constants::LANGUAGE_FILE_EXTENSION);
        path_str.ends_with(suffix.as_str()).and_option_from(|| {
            let cut_from = path_str.len() - suffix.len();
            Some(Location(path_str[..cut_from].to_string()))
        })
    }

    /// Obtains path (within a project context) to the file with this module.
    pub fn to_path(&self) -> file_manager_client::Path {
        // TODO [mwu] Extremely provisional. When multiple files support is
        //            added, needs to be fixed, if not earlier.
        let Location(string) = self;
        let result = format!("./{}.{}", string, constants::LANGUAGE_FILE_EXTENSION);
        file_manager_client::Path::new(result)
    }
}



// =========================
// === Module Controller ===
// =========================

shared! { Handle
    /// State data of the module controller.
    #[derive(Debug)]
    pub struct Controller {
        /// This module's location.
        location: Location,
        /// The current module ast, used by synchronizing both module representations.
        ast: Ast,
        /// The id map of current ast
        // TODO: written for test purposes, should be removed once generating id_map from AST will
        // be implemented.
        id_map: IdMap,
        /// The File Manager Client handle.
        file_manager: fmc::Handle,
        /// The Parser handle.
        parser: Parser,
        /// Publisher of "text changed" notifications
        text_notifications  : notification::TextChangedPublisher,
        /// Publisher of "graph changed" notifications
        graph_notifications : notification::GraphChangedPublisher,
        /// The logger handle.
        logger: Logger,
    }

    impl {
        /// Obtain clone of location.
        pub fn location(&self) -> Location {
            self.location.clone()
        }

        /// Updates AST after code change.
        pub fn apply_code_change(&mut self,change:&TextChangedInfo) -> FallibleResult<()> {
            let mut code        = self.code();
            let replaced_range  = change.replaced_chars.clone();
            let inserted_string = change.inserted_string();
            let replaced_size   = Size::new(replaced_range.end - replaced_range.start);
            let replaced_span   = Span::new(Index::new(replaced_range.start),replaced_size);

            code.replace_range(replaced_range,&inserted_string);
            apply_code_change_to_id_map(&mut self.id_map,&replaced_span,&inserted_string);
            self.ast = self.parser.parse(code, self.id_map.clone())?;
            self.logger.trace(|| format!("Applied change; Ast is now {:?}", self.ast));

            let text_change  = notification::TextChanged::Entirely;
            let graph_change = notification::GraphChanged::Entirely;
            let code_notify  = self.text_notifications.publish(text_change);
            let graph_notify = self.graph_notifications.publish(graph_change);
            spawn(async move { futures::join!(code_notify,graph_notify); });

            Ok(())
        }

        /// Read module code.
        pub fn code(&self) -> String {
            self.ast.repr()
        }

        /// Check if current module state is synchronized with given code. If it's not, log error,
        /// and update module state to match the `code` passed as argument.
        pub fn check_code_sync(&mut self, code:String) -> FallibleResult<()> {
            let my_code = self.code();
            if code != my_code {
                self.logger.error(|| format!("The module controller ast was not synchronized with \
                    text editor content!\n >>> Module: {:?}\n >>> Editor: {:?}",my_code,code));
                self.ast    = self.parser.parse(code,default())?;
                self.id_map = default();
            }
            Ok(())
        }

        /// Get subscriber receiving notifications about changes in module's text representation.
        pub fn subscribe_text_notifications(&mut self) -> Subscriber<notification::TextChanged> {
            self.text_notifications.subscribe()
        }

        /// Get subscriber receiving notifications about changes in module's graph representation.
        pub fn subscribe_graph_notifications(&mut self) -> Subscriber<notification::GraphChanged> {
            self.graph_notifications.subscribe()
        }
    }
}

impl Handle {
    /// Create a module controller for given location.
    ///
    /// It may wait for module content, because the module must initialize its state.
    pub async fn new(location:Location, mut file_manager:fmc::Handle, parser:Parser)
    -> FallibleResult<Self> {
        let logger              = Logger::new(format!("Module Controller {}", location));
        let path                = location.to_path();
        let ast                 = Ast::new(ast::Blank{},None);
        let id_map              = default();
        let text_notifications  = default();
        let graph_notifications = default();

        file_manager.touch(path).await?;
        let data = Controller {location,ast,file_manager,parser,id_map,logger,text_notifications,
            graph_notifications};
        let handle = Handle::new_from_data(data);
        handle.load_file().await?;
        Ok(handle)
    }

    /// Load or reload module content from file.
    pub async fn load_file(&self) -> FallibleResult<()> {
        let (logger,path,mut fm,mut parser) = self.with_borrowed(|data| {
            ( data.logger.clone()
            , data.location.to_path()
            , data.file_manager.clone_ref()
            , data.parser.clone_ref()
            )
        });
        logger.info(|| "Loading module file");
        let content = fm.read(path).await?;
        logger.info(|| "Parsing code");
        let ast = parser.parse(content,default())?;
        logger.info(|| "Code parsed");
        logger.trace(|| format!("The parsed ast is {:?}", ast));
        self.with_borrowed(|data| data.ast = ast);
        Ok(())
    }

    /// Save the module to file.
    pub fn save_file(&self) -> impl Future<Output=Result<(),RpcError>> {
        let (path,mut fm) = self.with_borrowed(|data| {
            (data.location.to_path(),data.file_manager.clone_ref())
        });
        fm.write(path,self.code())
        // TODO [ao] here save also the id_map and metadata.
    }

    #[cfg(test)]
    fn new_mock
    (location:Location, code:&str, id_map:IdMap, file_manager:fmc::Handle, mut parser:Parser)
    -> FallibleResult<Self> {
        let logger              = Logger::new("Mocked Module Controller");
        let ast                 = parser.parse(code.to_string(),id_map.clone())?;
        let text_notifications  = default();
        let graph_notifications = default();
        let data                = Controller {location,ast,file_manager,parser,id_map,logger,
            text_notifications,graph_notifications};
        Ok(Handle::new_from_data(data))
    }
}



#[cfg(test)]
mod test {
    use super::*;

    use ast;
    use ast::BlockLine;
    use data::text::Index;
    use data::text::Span;
    use data::text::Size;
    use data::text::TextChange;
    use data::text::TextLocation;
    use json_rpc::test_util::transport::mock::MockTransport;
    use parser::Parser;
    use uuid::Uuid;
    use wasm_bindgen_test::wasm_bindgen_test;
    use file_manager_client::Path;
    use crate::executor::test_utils::TestWithLocalPoolExecutor;
    use crate::controller::notification::{TextChanged, GraphChanged};

    #[test]
    fn get_location_from_path() {
        let module     = Path(format!("test.{}", constants::LANGUAGE_FILE_EXTENSION));
        let not_module = Path("test.txt".to_string());

        let expected_loc = Location("test".to_string());
        assert_eq!(Some(expected_loc),Location::from_path(&module    ));
        assert_eq!(None,              Location::from_path(&not_module));
    }

    #[wasm_bindgen_test]
    fn update_ast_after_text_change() {
        TestWithLocalPoolExecutor::set_up().run_test(async {
            let transport    = MockTransport::new();
            let file_manager = file_manager_client::Handle::new(transport);
            let parser       = Parser::new().unwrap();
            let location     = Location("Test".to_string());

            let uuid1        = Uuid::new_v4();
            let uuid2        = Uuid::new_v4();
            let code         = "2+2";
            let id_map       = IdMap(vec!
            [ (Span::new(Index::new(0), Size::new(1)),uuid1.clone())
            , (Span::new(Index::new(2), Size::new(1)),uuid2)
            ]);

            let controller   = Handle::new_mock(location,code,id_map,file_manager,parser).unwrap();

            let mut text_notifications  = controller.subscribe_text_notifications();
            let mut graph_notifications = controller.subscribe_graph_notifications();

            // Change code from "2+2" to "22+2"
            let change = TextChangedInfo {
                change        : TextChange::insert(TextLocation{line:0,column:1}, "2"),
                replaced_chars: 1..1
            };
            controller.apply_code_change(&change).unwrap();
            let expected_ast = Ast::new(ast::Module {
                lines: vec![BlockLine {
                    elem: Some(Ast::new(ast::Infix {
                        larg : Ast::new(ast::Number{base:None, int:"22".to_string()}, Some(uuid1)),
                        loff : 0,
                        opr  : Ast::new(ast::Opr {name:"+".to_string()}, None),
                        roff : 0,
                        rarg : Ast::new(ast::Number{base:None, int:"2".to_string()}, Some(uuid2)),
                    }, None)),
                    off: 0
                }]
            }, None);
            assert_eq!(expected_ast, controller.with_borrowed(|data| data.ast.clone()));

            // Check emitted notifications
            assert_eq!(Some(TextChanged::Entirely ), text_notifications.next().await );
            assert_eq!(Some(GraphChanged::Entirely), graph_notifications.next().await);
        });
    }
}
