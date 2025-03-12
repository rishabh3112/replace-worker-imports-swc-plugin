use std::collections::HashMap;

use swc_core::atoms::Atom;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::*,
    transforms::testing::test_inline,
    visit::{visit_mut_pass, VisitMut, VisitMutWith},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

pub struct TransformVisitor {
    pub import_vs_path: HashMap<String, String>,
}

impl VisitMut for TransformVisitor {
    fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
        stmts.visit_mut_children_with(self);

        stmts.retain(|s| match s {
            ModuleItem::ModuleDecl(ModuleDecl::Import(x)) => {
                !x.src.is_empty() && !x.src.value.ends_with(".worker")
            }
            ModuleItem::Stmt(Stmt::Empty(..)) => false,
            _ => true,
        });
    }

    fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
        let path: String = node.src.value.clone().as_str().into();
        if !path.ends_with(".worker") {
            return;
        }

        for specifier in &node.specifiers {
            match specifier {
                ImportSpecifier::Default(import_default_specifier) => {
                    self.import_vs_path.insert(
                        import_default_specifier.local.clone().sym.as_str().into(),
                        path.clone(),
                    );
                }
                _ => continue,
            }
        }

        return;
    }

    fn visit_mut_new_expr(&mut self, node: &mut NewExpr) {
        match &*node.callee {
            Expr::Ident(ident) => {
                let id: &String = &ident.sym.as_str().into();
                if self.import_vs_path.contains_key(id) {
                    let path = self.import_vs_path.get(id).unwrap();

                    *node = NewExpr {
                        callee: Box::new(Expr::Ident(Ident::new(
                            Atom::from("Worker"),
                            ident.span,
                            ident.ctxt,
                        ))),
                        span: node.span,
                        ctxt: node.ctxt,
                        args: Option::Some(vec![
                            ExprOrSpread {
                                expr: Box::new(Expr::Lit(Lit::Str(Str::from(path.as_str())))),
                                spread: Option::None,
                            },
                            ExprOrSpread {
                                expr: Box::new(Expr::Member(MemberExpr {
                                    span: DUMMY_SP,
                                    obj: Box::new(Expr::Member(MemberExpr {
                                        span: DUMMY_SP,
                                        obj: Box::new(Expr::Ident(Ident::new(
                                            Atom::from("import"),
                                            DUMMY_SP,
                                            ident.ctxt,
                                        ))),
                                        prop: MemberProp::Ident(IdentName::new(
                                            Atom::from("meta"),
                                            DUMMY_SP,
                                        )),
                                    })),
                                    prop: MemberProp::Ident(IdentName::new(
                                        Atom::from("url"),
                                        DUMMY_SP,
                                    )),
                                })),
                                spread: Option::None,
                            },
                        ]),
                        type_args: Option::None,
                    };
                }
            }
            _ => {}
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    let mut program = program;
    program.visit_mut_with(&mut visit_mut_pass(TransformVisitor {
        import_vs_path: HashMap::new(),
    }));
    program
}

test_inline!(
    Default::default(),
    |_| visit_mut_pass(TransformVisitor {
        import_vs_path: HashMap::new(),
    }),
    boo,
    r#"
    import { useMemo } from "react";
    import SimulationWorker from "./simulation.worker";
    const worker = new SimulationWorker();
    "#,
    r#"
    import { useMemo } from "react";
    const worker = new Worker("./simulation.worker", import.meta.url);
    "#
);
