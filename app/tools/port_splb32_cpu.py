# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Reproduce the direct CC0 BrickEmuPy SPLB32 CPU translation; no Tama-Go code."""
import ast
from pathlib import Path
from workspace_paths import references_dir
ROOT=Path(__file__).resolve().parents[2]
SOURCE=references_dir()/"BrickEmuPy-main/cores/SPLB32.py"

def field(name):
    return {"_IF":"iflag"}.get(name,name.lstrip("_").lower())
OPS={ast.Add:"+",ast.Sub:"-",ast.Mult:"*",ast.BitAnd:"&",ast.BitOr:"|",ast.BitXor:"^",ast.LShift:"<<",ast.RShift:">>",ast.Mod:"%"}
CMP={ast.Gt:">",ast.GtE:">=",ast.Lt:"<",ast.LtE:"<=",ast.Eq:"==",ast.NotEq:"!="}
def expr(n):
    if isinstance(n,ast.Constant):return str(int(n.value))
    if isinstance(n,ast.Name):return {"SP":"256","SFR_NMI_CTRL_ENABLE":"128"}.get(n.id,n.id.lower())
    if isinstance(n,ast.Attribute):return "self."+field(n.attr)
    if isinstance(n,ast.BinOp):return f"({expr(n.left)} {OPS[type(n.op)]} {expr(n.right)})"
    if isinstance(n,ast.UnaryOp):return f"i64::from({expr(n.operand)} == 0)" if isinstance(n.op,ast.Not) else f"(!{expr(n.operand)})"
    if isinstance(n,ast.Compare):return f"i64::from({expr(n.left)} {CMP[type(n.ops[0])]} {expr(n.comparators[0])})"
    if isinstance(n,ast.BoolOp):return "i64::from("+(" && " if isinstance(n.op,ast.And)else" || ").join("("+expr(x)+") != 0"for x in n.values)+")"
    if isinstance(n,ast.Call):return expr(n.func)+"("+", ".join(expr(a)for a in n.args)+")"
    raise ValueError(ast.dump(n))
def statements(nodes,known):
    out=[]
    for n in nodes:
        if isinstance(n,ast.Assign):
            assert len(n.targets)==1;t=n.targets[0];decl=""
            if isinstance(t,ast.Name)and t.id not in known:decl="let mut ";known.add(t.id)
            out.append(decl+expr(t)+" = "+expr(n.value)+";")
        elif isinstance(n,ast.AugAssign):out.append(expr(n.target)+" "+OPS[type(n.op)]+"= "+expr(n.value)+";")
        elif isinstance(n,ast.Return):out.append("return "+expr(n.value)+";")
        elif isinstance(n,ast.Expr):out.append(expr(n.value)+";")
        elif isinstance(n,ast.If):out.append("if ("+expr(n.test)+") != 0 {\n"+statements(n.body,set(known))+"}\n"+("else {\n"+statements(n.orelse,set(known))+"}\n"if n.orelse else""))
        elif isinstance(n,ast.Pass):pass
        else:raise ValueError(ast.dump(n))
    return "\n".join(out)+"\n"
def flatten(n):
    if isinstance(n,ast.Attribute):return [field(n.attr)]
    if isinstance(n,(ast.Tuple,ast.List)):return [v for x in n.elts for v in flatten(x)]
    if isinstance(n,ast.Starred):return flatten(n.value)
    if isinstance(n,ast.BinOp)and isinstance(n.op,ast.Mult):return flatten(n.left)*n.right.value
    raise ValueError(ast.dump(n))
def generate():
    module=ast.parse(SOURCE.read_text());cls=next(n for n in module.body if isinstance(n,ast.ClassDef));init=next(n for n in cls.body if isinstance(n,ast.FunctionDef)and n.name=="__init__")
    table=next(flatten(n.value)for n in init.body if isinstance(n,ast.Assign)and isinstance(n.targets[0],ast.Attribute)and n.targets[0].attr=="_execute");assert len(table)==256
    funcs=[n for n in cls.body if isinstance(n,ast.FunctionDef)];start=next(i for i,n in enumerate(funcs)if n.name=="_get_ps")
    chosen=[n for n in funcs[start:]if n.name!="_dummy"]+[n for n in funcs if n.name in ("_IRQ","_NMI")]
    out="// SPDX-License-Identifier: GPL-3.0-only\n// Rust translation of azya52/BrickEmuPy SPLB32.py (CC0-1.0).\n// Reproduce with tools/port_splb32_cpu.py. Original reference behavior is retained.\n#![allow(unused_parens,unused_mut)]\nuse super::Machine;\nimpl Machine {\n"
    for f in chosen:
        returns=any(isinstance(n,ast.Return)for n in ast.walk(f));args=f.args.args[1:];known={a.arg for a in args}
        out+="pub(super) fn "+field(f.name)+"(&"+(""if f.name=="_get_ps"else"mut ")+"self"+"".join(", "+a.arg+":i64"for a in args)+")"+("-> i64"if returns else"")+" {\n"+statements(f.body,known)+"}\n"
    out+='pub fn step_cpu(&mut self)->Result<i64,String>{let opcode=self.read_mem(self.pc);match opcode{\n'
    for i,name in enumerate(table):
        if name!="dummy":out+=f"{i}=>{{self.pc=(self.pc+1)&65535;Ok(self.{name}())}},\n"
    out+='_=>Err(format!("Unsupported SPLB32 opcode {opcode:02X} at {:05X}",self.physical_pc()))}}\n}\n'
    return out
if __name__=="__main__":
    import argparse,subprocess
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true");args=parser.parse_args()
    text=subprocess.run(["rustfmt","--edition","2024"],input=generate(),text=True,capture_output=True,check=True).stdout
    dest=ROOT/"app/src/core/splb32/cpu.rs"
    if args.check:assert dest.read_text()==text,"Stale direct SPLB32 CPU translation"
    else:dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text,encoding="utf-8")
    print("Verified direct CC0 SPLB32 CPU translation")
