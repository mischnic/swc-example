use swc_atoms::JsWord;
use swc_common::{
    chain, comments::SingleThreadedComments, sync::Lrc, FileName, Globals, Mark, SourceMap,
};
use swc_ecmascript::visit::FoldWith;
use swc_ecmascript::{
    ast::*,
    codegen::text_writer::JsWriter,
    parser::{lexer::Lexer, EsConfig, PResult, Parser, StringInput, Syntax},
    transforms::{compat::es3::reserved_words, fixer, hygiene, resolver},
    visit::Fold,
};

// use swc_common::{errors::{Emitter, DiagnosticBuilder}};
// #[derive(Debug, Clone, Default)]
// pub struct ErrorBuffer(std::sync::Arc<std::sync::Mutex<Vec<swc_common::errors::Diagnostic>>>);

// impl Emitter for ErrorBuffer {
//     fn emit(&mut self, db: &DiagnosticBuilder) {
//         self.0.lock().unwrap().push((**db).clone());
//     }
// }

fn main() {
    let src = r#"
    console.log("hello");
"#;
    let cm = Lrc::<SourceMap>::default();
    let (program, comments) = parse(src, "test.js", &cm).unwrap();

    // let error_buffer = ErrorBuffer::default();
    // let handler = Handler::with_emitter(true, false, Box::new(error_buffer.clone()));
    swc_common::GLOBALS.set(&Globals::new(), || {
        // swc_common::errors::HANDLER.set(&handler, || {
        // helpers::HELPERS.set(&helpers::Helpers::new(true), || {
        let unresolved_mark = Mark::fresh(Mark::root());
        let top_level_mark = Mark::fresh(Mark::root());
        let program = program.fold_with(&mut resolver(unresolved_mark, top_level_mark, false));

        // alternatively:
        // program.visit_with with an `impl Visit for ..`
        // program.visit_mut_with  with an `impl VisitMut for ..`
        let program = program.fold_with(&mut ExposeSyntaxContext {
            top_level_mark,
            unresolved_mark,
        });

        let program = program.fold_with(&mut chain!(
            reserved_words(true),
            hygiene(),
            fixer(Some(&comments))
        ));

        // for diagnostic in error_buffer.0.lock().unwrap().clone() {
        //     let message = diagnostic.message();
        //     let span = diagnostic.span.clone();
        //     println!("-- Diagnostic --\nMessage:{}", message);

        //     let span_labels = span.span_labels();
        //     if !span_labels.is_empty() {
        //         for span_label in span_labels {
        //             let start =
        // cm.lookup_char_pos(span_label.span.lo);
        //             let end = cm.lookup_char_pos(span_label.span.hi);
        //             println!("{:?} - {:?} :: {:?}", start, end,
        // span_label.label);         }
        //     };
        // }

        let code = emit(&program, &comments, cm);
        println!("{}", code);
        // });
        // });
    });
}

struct ExposeSyntaxContext {
    top_level_mark: Mark,
    unresolved_mark: Mark,
}
impl Fold for ExposeSyntaxContext {
    fn fold_ident(&mut self, node: Ident) -> Ident {
        let new_name: JsWord = format!(
            "{}_{}{}{}",
            node.sym,
            node.span.ctxt().as_u32(),
            if node.span.has_mark(self.top_level_mark) {
                "_top"
            } else {
                ""
            },
            if node.span.has_mark(self.unresolved_mark) {
                "_unres"
            } else {
                ""
            },
        )
        .into();
        Ident::new(new_name, node.span)
    }
}

fn parse(
    code: &str,
    filename: &str,
    cm: &Lrc<SourceMap>,
) -> PResult<(Program, SingleThreadedComments)> {
    let source_file = cm.new_source_file(FileName::Real(filename.into()), code.into());
    let comments = SingleThreadedComments::default();

    let lexer = Lexer::new(
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*source_file),
        Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);
    match parser.parse_program() {
        Err(err) => Err(err),
        Ok(program) => Ok((program, comments)),
    }
}

fn emit(program: &Program, comments: &SingleThreadedComments, cm: Lrc<SourceMap>) -> String {
    let mut buf = vec![];
    {
        let writer = Box::new(JsWriter::new(cm.clone(), "\n", &mut buf, None));
        let config = swc_ecmascript::codegen::Config {
            minify: false,
            ..Default::default()
        };
        let mut emitter = swc_ecmascript::codegen::Emitter {
            cfg: config,
            comments: Some(&comments),
            cm,
            wr: writer,
        };
        emitter.emit_program(program).unwrap();
    }

    String::from_utf8(buf).unwrap()
}
