//! This module contains all structures related to Searcher Controller.

use crate::prelude::*;

use crate::notification;

use data::text::TextLocation;
use enso_protocol::language_server;
use flo_stream::Subscriber;
use parser::Parser;


// =======================
// === Suggestion List ===
// =======================

/// Suggestion for input completion: possible functions, arguments, etc.
pub type CompletionSuggestion = Rc<model::suggestion_database::Entry>;

/// A single suggestion on the Searcher suggestion list.
#[derive(Clone,CloneRef,Debug,Eq,PartialEq)]
pub enum Suggestion {
    /// Suggestion for input completion: possible functions, arguments, etc.
    Completion(CompletionSuggestion)
    // In future, other suggestion types will be added (like suggestions of actions, etc.).
}

/// List of suggestions available in Searcher.
#[derive(Clone,CloneRef,Debug)]
pub enum Suggestions {
    /// The suggestion list is still loading from the Language Server.
    Loading,
    /// The suggestion list is loaded.
    #[allow(missing_docs)]
    Loaded {
        list : Rc<Vec<Suggestion>>
    },
    /// Loading suggestion list resulted in error.
    Error(Rc<failure::Error>)
}

impl Suggestions {
    /// Check if suggestion list is still loading.
    pub fn is_loading(&self) -> bool {
        match self {
            Self::Loading => true,
            _             => false,
        }
    }

    /// Check if retrieving suggestion list was unsuccessful
    pub fn is_error(&self) -> bool {
        match self {
            Self::Error(_) => true,
            _              => false,
        }
    }

    /// Get the list of suggestions. Returns None if still loading or error was returned.
    pub fn list(&self) -> Option<&Vec<Suggestion>> {
        match self {
            Self::Loaded {list} => Some(list),
            _                   => None,
        }
    }
}

impl Default for Suggestions {
    fn default() -> Self {
        Self::Loading
    }
}



// =====================
// === Notifications ===
// =====================

/// The notification emitted by Searcher Controller
#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Notification {
    /// A new Suggestion list is available.
    NewSuggestionList
}


// ===================
// === Input Parts ===
// ===================

/// A Searcher Input which is parsed to the _expression_ and _pattern_ parts.
///
/// We parse the input for better understanding what user wants to add.
#[derive(Clone,Debug,Default)]
pub struct ParsedInput {
    /// The part of input which is treated as completed function and some set of arguments.
    ///
    /// The expression is kept as prefix chain, as it allows us to easily determine what kind of
    /// entity we can put at this moment (is it a function or argument? What type of the argument?).
    pub expression : Option<ast::Shifted<ast::prefix::Chain>>,
    /// An offset between expression and pattern.
    pub pattern_offset : usize,
    /// The part of input being a function/argument which is still typed by user. It is used
    /// for filtering suggestions.
    pub pattern : String,
}

impl ParsedInput {
    /// Contructor from the plain input.
    fn new(mut input:String, parser:&Parser) -> FallibleResult<Self> {
        let leading_spaces = input.chars().take_while(|c| *c == ' ').count();
        // To properly guess what is "still typed argument" we simulate type of one letter by user.
        // This letter will be added to the last argument (or function if there is no argument), or
        // will be a new argument (so the user starts filling a new argument).
        //
        // See also `parsed_input` test to see all cases we want to cover.
        input.push('a');
        let ast        = parser.parse_line(input.trim_start())?;
        let mut prefix = ast::prefix::Chain::new_non_strict(&ast);
        let last_arg   = prefix.args.pop();
        if let Some(last_arg) = last_arg {
            let mut last_arg_repr = last_arg.sast.wrapped.repr();
            last_arg_repr.pop();
            Ok(ParsedInput {
                expression     : Some(ast::Shifted::new(leading_spaces,prefix)),
                pattern_offset : last_arg.sast.off,
                pattern        : last_arg_repr,
            })
        } else {
            let mut func_repr = prefix.func.repr();
            func_repr.pop();
            Ok(ParsedInput {
                expression     : None,
                pattern_offset : leading_spaces,
                pattern        : func_repr
            })
        }
    }

    /// Returns the id of the next fragment potentially filled by picking completion suggestion.
    fn next_completion_id(&self) -> CompletedFragmentId {
        match &self.expression {
            None             => CompletedFragmentId::Function,
            Some(expression) => CompletedFragmentId::Argument {id:expression.args.len()},
        }
    }
}

impl HasRepr for ParsedInput {
    fn repr(&self) -> String {
        let expr          = self.expression.as_ref().map_or("".to_string(), HasRepr::repr);
        let offset:String = std::iter::repeat(' ').take(self.pattern_offset).collect();
        iformat!("{expr}{offset}{self.pattern}")
    }
}



// ===========================
// === Searcher Controller ===
// ===========================

/// An identification of input fragment filled by picking suggestion.
#[derive(Clone,Copy,Debug,Eq,PartialEq)]
#[allow(missing_docs)]
pub enum CompletedFragmentId {
    /// The called "function" part, defined as a `func` element in Prefix Chain
    /// (see `ast::prefix::Chain`).
    Function,
    /// The `id`th argument of the called function.
    Argument{id:usize}
}

impl CompletedFragmentId {
    /// Get the picked fragment from the Searcher's input.
    pub fn completed_fragment(&self,input:&ParsedInput) -> Option<String> {
        use CompletedFragmentId::*;
        match (self,&input.expression) {
            (_           ,None)       => None,
            (Function    ,Some(expr)) => Some(expr.func.repr()),
            (Argument{id},Some(expr)) => Some(expr.args.get(*id)?.sast.wrapped.repr()),
        }
    }
}

/// A fragment filled by single picked completion suggestion.
///
/// We store such information in Searcher to better suggest the potential arguments, and to know
/// what imports should be added when inserting node.
#[derive(Clone,Debug)]
#[allow(missing_docs)]
pub struct FragmentAddedByPickingSuggestion {
    pub id                : CompletedFragmentId,
    pub picked_suggestion : CompletionSuggestion,
}

impl FragmentAddedByPickingSuggestion {
    /// Check if the picked fragment is still unmodified by user.
    fn still_unmodified(&self, input:&ParsedInput) -> bool {
        self.id.completed_fragment(input).as_ref() == Some(&self.picked_suggestion.name)
    }
}

/// A controller state.
#[derive(Clone,Debug,Default)]
pub struct Data {
    /// The current searcher's input.
    pub current_input : ParsedInput,
    /// The suggestion list which should be displayed.
    pub current_list : Suggestions,
    /// All fragments of input which were added by picking suggestions. If the fragment will be
    /// changed by user, it will be removed from this list.
    pub fragments_added_by_picking : Vec<FragmentAddedByPickingSuggestion>,
}

/// Searcher Controller.
///
/// This is an object providing all required functionalities for Searcher View: mainly it is the
/// suggestion list to display depending on the searcher input, and actions of picking one or
/// accepting the Searcher input (pressing "Enter").
#[derive(Clone,CloneRef,Debug)]
pub struct Searcher {
    logger          : Logger,
    data            : Rc<RefCell<Data>>,
    notifier        : notification::Publisher<Notification>,
    module          : Rc<model::module::QualifiedName>,
    position        : Immutable<TextLocation>,
    database        : Rc<model::SuggestionDatabase>,
    language_server : Rc<language_server::Connection>,
    parser          : Parser,
}

impl Searcher {
    /// Create new Searcher Controller.
    pub fn new
    ( parent   : impl AnyLogger
    , project  : &model::Project
    , module   : model::module::Path
    , position : TextLocation
    ) -> Self {
        let this = Self {
            position        : Immutable(position),
            logger          : Logger::sub(parent,"Searcher Controller"),
            data            : default(),
            notifier        : default(),
            module          : Rc::new(project.qualified_module_name(&module)),
            database        : project.suggestion_db.clone_ref(),
            language_server : project.language_server_rpc.clone_ref(),
            parser          : project.parser.clone_ref(),
        };
        this.reload_list();
        this
    }

    /// Subscribe to controller's notifications.
    pub fn subscribe(&self) -> Subscriber<Notification> {
        self.notifier.subscribe()
    }

    /// Get the current suggestion list.
    pub fn suggestions(&self) -> Suggestions {
        self.data.borrow().current_list.clone_ref()
    }

    /// Set the Searcher Input.
    ///
    /// This function should be called each time user modifies Searcher input in view. It may result
    /// in a new suggestion list (the aprriopriate notification will be emitted).
    pub fn set_input(&self, new_input:String) -> FallibleResult<()> {
        let parsed_input = ParsedInput::new(new_input,&self.parser)?;
        let old_id       = self.data.borrow().current_input.next_completion_id();
        let new_id       = parsed_input.next_completion_id();

        self.data.borrow_mut().current_input = parsed_input;
        self.invalidate_fragments_added_by_picking();
        if old_id != new_id {
            self.reload_list()
        }
        Ok(())
    }

    /// User picked a completion suggestion.
    ///
    /// This function should be called when user chooses some completion suggestion. It still will
    /// be editing the input. The picked suggestion will be remembered,
    pub fn pick_completion(&self, picked_suggestion:CompletionSuggestion) -> String {
        let added_ast         = ast::Ast::var(&picked_suggestion.name);
        let id                = self.data.borrow().current_input.next_completion_id();
        let picked_completion = FragmentAddedByPickingSuggestion {id,picked_suggestion};
        let pattern_offset    = self.data.borrow().current_input.pattern_offset;
        let new_expression    = match self.data.borrow_mut().current_input.expression.take() {
            None => {
                let ast = ast::prefix::Chain::new_non_strict(&added_ast);
                ast::Shifted::new(pattern_offset,ast)
            },
            Some(mut expression) => {
                let new_argument = ast::prefix::Argument {
                    sast      : ast::Shifted::new(pattern_offset,added_ast),
                    prefix_id : default(),
                };
                expression.args.push(new_argument);
                expression
            }
        };
        let new_parsed_input = ParsedInput {
            expression     : Some(new_expression),
            pattern_offset : 1,
            pattern        : "".to_string()
        };
        let new_input = new_parsed_input.repr();
        self.data.borrow_mut().current_input = new_parsed_input;
        self.data.borrow_mut().fragments_added_by_picking.push(picked_completion);
        self.reload_list();
        new_input
    }

    fn invalidate_fragments_added_by_picking(&self) {
        let mut data = self.data.borrow_mut();
        let data     = data.deref_mut();
        let input    = &data.current_input;
        data.fragments_added_by_picking.drain_filter(|frag| !frag.still_unmodified(input));
    }

    /// Reload Suggestion List.
    ///
    /// The current list will be set as "Loading" and Language Server will be requested for a new
    /// list - once it be retrieved, the new list will be set and notification will be emitted.
    fn reload_list(&self) {
        let next_completion = self.data.borrow().current_input.next_completion_id();
        let new_list = if next_completion == CompletedFragmentId::Function {
            self.get_suggestion_list_from_engine(None,None);
            Suggestions::Loading
        } else {
            // TODO[ao] Requesting for argument.
            Suggestions::Loaded {list:default()}
        };
        self.data.borrow_mut().current_list = new_list;
    }

    fn get_suggestion_list_from_engine
    (&self, return_type:Option<String>, tags:Option<Vec<language_server::SuggestionEntryType>>) {
        let ls          = &self.language_server;
        let module      = self.module.as_ref();
        let self_type   = None;
        let position    = self.position.deref().into();
        let request     = ls.completion(module,&position,&self_type,&return_type,&tags);
        let data        = self.data.clone_ref();
        let database    = self.database.clone_ref();
        let logger      = self.logger.clone_ref();
        let notifier    = self.notifier.clone_ref();
        executor::global::spawn(async move {
            info!(logger,"Requesting new suggestion list.");
            let response = request.await;
            info!(logger,"Received suggestions from Language Server.");
            let new_list = match response {
                Ok(list) => {
                    let entry_ids   = list.results.into_iter();
                    let entries     = entry_ids.filter_map(|id| {
                        let entry = database.get(id);
                        if entry.is_none() {
                            error!(logger,"Missing entry {id} in Suggestion Database.");
                        }
                        entry
                    });
                    let suggestions = entries.map(Suggestion::Completion);
                    Suggestions::Loaded {list:Rc::new(suggestions.collect())}
                },
                Err(error) => Suggestions::Error(Rc::new(error.into()))
            };
            data.borrow_mut().current_list = new_list;
            notifier.publish(Notification::NewSuggestionList).await;
        });
    }
}


// =============
// === Tests ===
// =============

#[cfg(test)]
mod test {
    use super::*;

    use crate::executor::test_utils::TestWithLocalPoolExecutor;
    use crate::model::module::Path;

    use json_rpc::expect_call;
    use utils::test::traits::*;

    struct Fixture {
        searcher : Searcher,
        entry1   : CompletionSuggestion,
        entry2   : CompletionSuggestion,
        entry9   : CompletionSuggestion,
    }

    impl Fixture {
        fn new(client_setup:impl FnOnce(&mut language_server::MockClient)) -> Self {
            let mut client  = language_server::MockClient::default();
            let module_path = Path::from_mock_module_name("Test");
            client_setup(&mut client);
            let searcher = Searcher {
                logger          : default(),
                data            : default(),
                notifier        : default(),
                module          : Rc::new(QualifiedName::from_path(&module_path,"Test")),
                position        : Immutable(TextLocation::at_document_begin()),
                database        : default(),
                language_server : language_server::Connection::new_mock_rc(client),
                parser          : Parser::new_or_panic(),
            };
            let entry1 = model::suggestion_database::Entry {
                name          : "TestFunction1".to_string(),
                kind          : model::suggestion_database::EntryKind::Function,
                module        : "Test.Test".to_string(),
                arguments     : vec![],
                return_type   : "Number".to_string(),
                documentation : default(),
                self_type     : None
            };
            let entry2 = model::suggestion_database::Entry {
                name : "TestVar1".to_string(),
                ..entry1.clone()
            };
            let entry9 = model::suggestion_database::Entry {
                name : "TestFunction2".to_string(),
                ..entry1.clone()
            };
            searcher.database.put_entry(1,entry1);
            let entry1 = searcher.database.get(1).unwrap();
            searcher.database.put_entry(2,entry2);
            let entry2 = searcher.database.get(2).unwrap();
            searcher.database.put_entry(9,entry9);
            let entry9 = searcher.database.get(9).unwrap();
            Fixture{searcher,entry1,entry2,entry9}
        }
    }


    #[wasm_bindgen_test]
    fn loading_list() {
        let mut test = TestWithLocalPoolExecutor::set_up();
        let Fixture{searcher,entry1,entry9,..} = Fixture::new(|client| {
            let completion_response = language_server::response::Completion {
                results: vec![1,5,9],
                current_version: default(),
            };
            expect_call!(client.completion(
                module      = "Test.Test".to_string(),
                position    = TextLocation::at_document_begin().into(),
                self_type   = None,
                return_type = None,
                tag         = None
            ) => Ok(completion_response));
        });

        let mut subscriber = searcher.subscribe();
        searcher.reload_list();
        assert!(searcher.suggestions().is_loading());
        test.run_until_stalled();
        let expected_list = vec![Suggestion::Completion(entry1),Suggestion::Completion(entry9)];
        assert_eq!(searcher.suggestions().list(), Some(&expected_list));
        let notification = subscriber.next().boxed_local().expect_ready();
        assert_eq!(notification, Some(Notification::NewSuggestionList));
    }

    #[wasm_bindgen_test]
    fn parsed_input() {
        let parser = Parser::new_or_panic();

        fn args_repr(prefix:&ast::prefix::Chain) -> Vec<String> {
            prefix.args.iter().map(|arg| arg.repr()).collect()
        }

        let input  = "";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        assert!(parsed.expression.is_none());
        assert_eq!(parsed.pattern.as_str(), "");

        let input  = "foo";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        assert!(parsed.expression.is_none());
        assert_eq!(parsed.pattern.as_str(), "foo");

        let input  = " foo";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        assert!(parsed.expression.is_none());
        assert_eq!(parsed.pattern_offset,   1);
        assert_eq!(parsed.pattern.as_str(), "foo");

        let input  = "foo  ";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        let expression = parsed.expression.unwrap();
        assert_eq!(expression.off         , 0);
        assert_eq!(expression.func.repr() , "foo");
        assert_eq!(args_repr(&expression) , Vec::<String>::new());
        assert_eq!(parsed.pattern_offset,   2);
        assert_eq!(parsed.pattern.as_str(), "");

        let input  = "foo bar";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        let expression = parsed.expression.unwrap();
        assert_eq!(expression.off         , 0);
        assert_eq!(expression.func.repr() , "foo");
        assert_eq!(args_repr(&expression) , Vec::<String>::new());
        assert_eq!(parsed.pattern_offset,   1);
        assert_eq!(parsed.pattern.as_str(), "bar");

        let input  = "foo  bar  baz";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        let expression = parsed.expression.unwrap();
        assert_eq!(expression.off         , 0);
        assert_eq!(expression.func.repr() , "foo");
        assert_eq!(args_repr(&expression) , vec!["  bar".to_string()]);
        assert_eq!(parsed.pattern_offset  , 2);
        assert_eq!(parsed.pattern.as_str(), "baz");

        let input  = "  foo bar baz ";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        let expression = parsed.expression.unwrap();
        assert_eq!(expression.off         , 2);
        assert_eq!(expression.func.repr() , "foo");
        assert_eq!(args_repr(&expression) , vec![" bar".to_string()," baz".to_string()]);
        assert_eq!(parsed.pattern_offset,   1);
        assert_eq!(parsed.pattern.as_str(), "");

        let input  = "foo bar (baz ";
        let parsed = ParsedInput::new(input.to_string(),&parser).unwrap();
        let expression = parsed.expression.unwrap();
        assert_eq!(expression.off         , 0);
        assert_eq!(expression.func.repr() , "foo");
        assert_eq!(args_repr(&expression) , vec![" bar".to_string()]);
        assert_eq!(parsed.pattern_offset,   1);
        assert_eq!(parsed.pattern.as_str(), "(baz ");
    }

    #[wasm_bindgen_test]
    fn picked_completions_list_maintaining() {
        let Fixture{searcher,entry1,entry2,..} = Fixture::new(|_|{});
        let frags_borrow = || Ref::map(searcher.data.borrow(),|d| &d.fragments_added_by_picking);

        // Picking first suggestion.
        let new_input = searcher.pick_completion(entry1.clone_ref());
        assert_eq!(new_input, "TestFunction1 ");
        let (func,) = frags_borrow().iter().cloned().expect_tuple();
        assert_eq!(func.id, CompletedFragmentId::Function);
        assert!(Rc::ptr_eq(&func.picked_suggestion,&entry1));

        // Typing more args by hand.
        searcher.set_input("TestFunction1 some_arg pat".to_string()).unwrap();
        let (func,) = frags_borrow().iter().cloned().expect_tuple();
        assert_eq!(func.id, CompletedFragmentId::Function);
        assert!(Rc::ptr_eq(&func.picked_suggestion,&entry1));

        // Picking argument's suggestion.
        let new_input = searcher.pick_completion(entry2.clone_ref());
        assert_eq!(new_input, "TestFunction1 some_arg TestVar1 ");
        let new_input = searcher.pick_completion(entry2.clone_ref());
        assert_eq!(new_input, "TestFunction1 some_arg TestVar1 TestVar1 ");
        let (function,arg1,arg2) = frags_borrow().iter().cloned().expect_tuple();
        assert_eq!(function.id, CompletedFragmentId::Function);
        assert!(Rc::ptr_eq(&function.picked_suggestion,&entry1));
        assert_eq!(arg1.id, CompletedFragmentId::Argument {id:1});
        assert!(Rc::ptr_eq(&arg1.picked_suggestion,&entry2));
        assert_eq!(arg2.id, CompletedFragmentId::Argument {id:2});
        assert!(Rc::ptr_eq(&arg2.picked_suggestion,&entry2));

        // Backspacing back to the second arg.
        searcher.set_input("TestFunction1 some_arg TestVar1 TestV".to_string()).unwrap();
        let (picked,arg) = frags_borrow().iter().cloned().expect_tuple();
        assert_eq!(picked.id, CompletedFragmentId::Function);
        assert!(Rc::ptr_eq(&picked.picked_suggestion,&entry1));
        assert_eq!(arg.id, CompletedFragmentId::Argument {id:1});
        assert!(Rc::ptr_eq(&arg.picked_suggestion,&entry2));

        // Editing the picked function.
        searcher.set_input("TestFunction2 some_arg TestVar1 TestV".to_string()).unwrap();
        let (arg,) = frags_borrow().iter().cloned().expect_tuple();
        assert_eq!(arg.id, CompletedFragmentId::Argument {id:1});
        assert!(Rc::ptr_eq(&arg.picked_suggestion,&entry2));
    }
}
