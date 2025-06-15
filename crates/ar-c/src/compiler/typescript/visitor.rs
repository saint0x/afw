//! The AST visitor for traversing the TypeScript AST.
//!
//! This module is responsible for identifying decorated entities (`@tool`, `@agent`)
//! and extracting their metadata and implementation source code.

use swc_ecma_ast::{Module, Expr, Lit, KeyValueProp, ClassDecl, FnDecl, ClassMethod};
use swc_ecma_visit::{Visit, VisitWith};

use crate::compiler::schema::{ToolManifest, AgentManifest, TeamManifest, PipelineManifest};
use std::collections::HashMap;

/// A temporary struct to hold data extracted by the visitor.
#[derive(Debug)]
pub enum ExtractedItem {
    Tool {
        manifest: ToolManifest,
    },
    Agent {
        manifest: AgentManifest,
    },
    Team {
        manifest: TeamManifest,
    },
    Pipeline {
        manifest: PipelineManifest,
    },
}

/// An AST visitor that extracts Aria-specific implementations and their spans.
pub struct AstVisitor {
    pub items: Vec<ExtractedItem>,
}

impl AstVisitor {
    /// Create a new visitor with the source code.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Entrypoint to start visiting a module.
    pub fn visit_module(&mut self, module: &Module) {
        module.visit_with(self);
    }

    fn parse_tool_decorator(&mut self, name: String, decorator: &swc_ecma_ast::Decorator) {
        let mut manifest = ToolManifest {
            name: name,
            description: String::new(),
            inputs: HashMap::new(),
        };

        if let Some(call) = decorator.expr.as_call() {
            if let Some(expr) = call.args.get(0) {
                if let Expr::Object(obj) = &*expr.expr {
                    for prop in &obj.props {
                        if let Some(kv) = prop.as_prop().and_then(|p| p.as_key_value()) {
                            let key = self.get_prop_key(kv);
                            match key.as_str() {
                                "name" => manifest.name = self.get_prop_value(kv),
                                "description" => manifest.description = self.get_prop_value(kv),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        self.items.push(ExtractedItem::Tool {
            manifest,
        });
    }

    fn parse_agent_decorator(&mut self, class: &ClassDecl, decorator: &swc_ecma_ast::Decorator) {
        let mut manifest = AgentManifest {
            name: class.ident.sym.to_string(),
            description: String::new(),
            tools: Vec::new(),
        };

        if let Some(call) = decorator.expr.as_call() {
            if let Some(expr) = call.args.get(0) {
                if let Expr::Object(obj) = &*expr.expr {
                    for prop in &obj.props {
                        if let Some(kv) = prop.as_prop().and_then(|p| p.as_key_value()) {
                            let key = self.get_prop_key(kv);
                            match key.as_str() {
                                "name" => manifest.name = self.get_prop_value(kv),
                                "description" => manifest.description = self.get_prop_value(kv),
                                "tools" => manifest.tools = self.get_tools_list(kv),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        self.items.push(ExtractedItem::Agent {
            manifest,
        });
    }

    fn parse_team_decorator(&mut self, class: &ClassDecl, decorator: &swc_ecma_ast::Decorator) {
        let mut manifest = TeamManifest {
            name: class.ident.sym.to_string(),
            description: String::new(),
            members: Vec::new(),
        };

        if let Some(call) = decorator.expr.as_call() {
            if let Some(expr) = call.args.get(0) {
                if let Expr::Object(obj) = &*expr.expr {
                    for prop in &obj.props {
                        if let Some(kv) = prop.as_prop().and_then(|p| p.as_key_value()) {
                            let key = self.get_prop_key(kv);
                            match key.as_str() {
                                "name" => manifest.name = self.get_prop_value(kv),
                                "description" => manifest.description = self.get_prop_value(kv),
                                "members" => manifest.members = self.get_string_array(kv),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        self.items.push(ExtractedItem::Team { manifest });
    }

    fn parse_pipeline_decorator(&mut self, class: &ClassDecl, decorator: &swc_ecma_ast::Decorator) {
        let mut manifest = PipelineManifest {
            name: class.ident.sym.to_string(),
            description: String::new(),
        };

        if let Some(call) = decorator.expr.as_call() {
            if let Some(expr) = call.args.get(0) {
                if let Expr::Object(obj) = &*expr.expr {
                    for prop in &obj.props {
                        if let Some(kv) = prop.as_prop().and_then(|p| p.as_key_value()) {
                            let key = self.get_prop_key(kv);
                            match key.as_str() {
                                "name" => manifest.name = self.get_prop_value(kv),
                                "description" => manifest.description = self.get_prop_value(kv),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        self.items.push(ExtractedItem::Pipeline { manifest });
    }

    fn get_prop_key(&self, kv: &KeyValueProp) -> String {
        match &kv.key {
            swc_ecma_ast::PropName::Ident(ident) => ident.sym.to_string(),
            swc_ecma_ast::PropName::Str(s) => s.value.to_string(),
            _ => "".to_string(),
        }
    }

    fn get_prop_value(&self, kv: &KeyValueProp) -> String {
        match &*kv.value {
            Expr::Lit(Lit::Str(s)) => s.value.to_string(),
            _ => "".to_string(),
        }
    }

    fn get_method_name(&self, method: &ClassMethod) -> Option<String> {
        match &method.key {
            swc_ecma_ast::PropName::Ident(ident) => Some(ident.sym.to_string()),
            _ => None,
        }
    }

    fn get_string_array(&self, kv: &KeyValueProp) -> Vec<String> {
        let mut items = Vec::new();
        if let Expr::Array(array_lit) = &*kv.value {
            for elem in &array_lit.elems {
                if let Some(expr) = elem {
                    if let Expr::Lit(Lit::Str(s)) = &*expr.expr {
                        items.push(s.value.to_string());
                    }
                }
            }
        }
        items
    }

    fn get_tools_list(&self, kv: &KeyValueProp) -> Vec<String> {
        let mut tools = Vec::new();
        if let Expr::Array(array_lit) = &*kv.value {
            for elem in &array_lit.elems {
                if let Some(expr) = elem {
                    if let Expr::Lit(Lit::Str(s)) = &*expr.expr {
                        tools.push(s.value.to_string());
                    }
                }
            }
        }
        tools
    }
}

/// Implement the `Visit` trait to hook into the AST traversal process.
impl<'ast> Visit for AstVisitor {
    fn visit_fn_decl(&mut self, func: &FnDecl) {
        for decorator in &func.function.decorators {
            if let Some(call) = decorator.expr.as_call() {
                if let Some(ident) = call.callee.as_expr().and_then(|e| e.as_ident()) {
                    if ident.sym.as_ref() == "tool" {
                        self.parse_tool_decorator(func.ident.sym.to_string(), decorator);
                        return;
                    }
                }
            }
        }
        func.visit_children_with(self);
    }

    fn visit_class_method(&mut self, method: &ClassMethod) {
        for decorator in &method.function.decorators {
            if let Some(call) = decorator.expr.as_call() {
                if let Some(ident) = call.callee.as_expr().and_then(|e| e.as_ident()) {
                    if ident.sym.as_ref() == "tool" {
                        if let Some(tool_name) = self.get_method_name(method) {
                            self.parse_tool_decorator(tool_name, decorator);
                        }
                        return; 
                    }
                }
            }
        }
        method.visit_children_with(self);
    }

    fn visit_class_decl(&mut self, class: &ClassDecl) {
        for decorator in &class.class.decorators {
            if let Some(call) = decorator.expr.as_call() {
                if let Some(ident) = call.callee.as_expr().and_then(|e| e.as_ident()) {
                    match ident.sym.as_ref() {
                        "agent" => {
                            self.parse_agent_decorator(class, decorator);
                            return; 
                        }
                        "team" => {
                            self.parse_team_decorator(class, decorator);
                            return;
                        }
                        "pipeline" => {
                            self.parse_pipeline_decorator(class, decorator);
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
        // If it's not a decorated class we care about, visit its children
        class.visit_children_with(self);
    }
} 