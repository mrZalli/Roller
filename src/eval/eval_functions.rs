use std::ops::{Add, Sub, Neg, Mul, Div};

use common_util::{Side, Pow};
use syntax_tree::*;
use eval::types::*;
use eval::env::*;
use error::{RollerErr, EvalErr, ParseResult};

macro_rules! expect_binary_op {
	($op: expr, $lhs: expr, $rhs: expr, $fun: path) => {
		match ($lhs, $rhs) {
			(Some(l_val), Some(r_val)) => $fun(l_val, r_val),

			(None, Some(_)) =>
				Err(RollerErr::EvalError(
					EvalErr::MissingOpArg{op: $op, side: Side::Left}
				)),
			(Some(_), None) =>
				Err(RollerErr::EvalError(
					EvalErr::MissingOpArg{op: $op, side: Side::Right}
				)),
			(None, None) =>
				Err(RollerErr::EvalError(
					EvalErr::NoOpArgs($op)
				)),
			}
	};
}


/// Evaluates a command and returns the value, if it was an expression, or returns an error.
pub fn eval_cmd(input: &Cmd, env: &mut RollerEnv) -> Option<ParseResult<Value>> {
	match input {
		&Cmd::Statement(ref s) =>
			match eval_stmt(s, env) {
				Ok(()) => None,
				Err(e) => Some(Err(e))
			},
		&Cmd::Expression(ref e) =>
			match eval_expr(e, env) {
				Ok(val) => Some(Ok(val)),
				Err(e) => Some(Err(e))
			}
	}
}

/// Evaluates a statement and returns an error if one was encountered.
pub fn eval_stmt(input: &Stmt, env: &mut RollerEnv) -> ParseResult<()> {
	match input {
		&Stmt::Assign(ref id, ref exp) => {
			let val = try!(eval_expr(exp, env));
			Ok(env.assign_var(id, val) )
		},
		&Stmt::FnDef(ref id, ref fun) =>
			Ok(env.declare_function(id, fun)),
		&Stmt::Delete(ref id) =>
			Ok(env.delete_id(id)),
		&Stmt::Clear =>
			Ok(env.clear()),
		/*Stmt::Run(path) =>
			,
		Stmt::Save(path) =>
			,*/
		_ => Err(RollerErr::EvalError(EvalErr::Unimplemented))
	}
}

/// Evaluates an expression and returns a value or error.
pub fn eval_expr(input: &Expr, env: &RollerEnv) -> ParseResult<Value> {
	match input {
		&Expr::Val(ref val) =>
			Ok(val.clone()),
		&Expr::List(ref vec_exp) =>
			Ok(Value::List(try!(eval_expr_vec(&vec_exp, env))) ),
		//Expr::Range{start, step, end} =>
		//	,
		&Expr::Var(ref id) =>
			env.get_var(id),
		&Expr::FunCall(ref id, ref args) =>
			env.call_fun(id, try!(eval_expr_vec(args, env))),

		&Expr::Op{op, ref left, ref right} if op == InfixOp::Dice => {
			let get_int_from_opt = |opt: &Option<Box<Expr>>|
				match opt {
					&None => Ok(1),
					&Some(ref exp) => match eval_expr(&*exp, env) {
						Ok(Value::Num(x)) if x.is_int() => Ok(x.as_int()),
						Ok(x) =>
							return Err(RollerErr::EvalError(
									EvalErr::ExpectedType{
										expected: RollerType::NumInt,
										found: RollerType::from(x),
									}
							)),
						Err(e) => Err(e)
					}
				};

			let n = try!(get_int_from_opt(left));
			let sides = try!(get_int_from_opt(right));

			if n < 0 {
				Err(RollerErr::EvalError(EvalErr::ExpectedPosNum(NumType::Int(n)) ))
			}
			else if sides < 0 {
				Err(RollerErr::EvalError(EvalErr::ExpectedPosNum(NumType::Int(sides)) ))
			}
			else {
				Ok(Value::List(env.get_roll(n as usize, sides) ))
			}
		},

		&Expr::Op{op, ref left, ref right} => {
			let eval_opt = |opt_exp: Option<Box<Expr>>|
				match opt_exp.map(|e| eval_expr(&*e, env)) {
					Some(Ok(v)) => Ok(Some(v)),
					None => Ok(None),
					Some(Err(e)) => Err(e)
				};
			eval_op(op, try!(eval_opt(left.clone() )), try!(eval_opt(right.clone() )) )
		},

		_ => Err(RollerErr::EvalError(EvalErr::Unimplemented))
	}
}

/// Evaluates a vector of expressions and stops at the first error.
fn eval_expr_vec(expr_vec: &Vec<Expr>, env: &RollerEnv) -> ParseResult<Vec<Value>> {
	let mut to_return = Vec::new();
	for i in expr_vec.into_iter() {
		match eval_expr(&i, env) {
			Ok(v) => to_return.push(v),
			Err(e) => return Err(e)
		}
	}
	return Ok(to_return);
}

/// Evaluates an infix operation
fn eval_op(op: InfixOp, lhs: Option<Value>, rhs: Option<Value>) -> ParseResult<Value> {
	match op {
		InfixOp::Dice => unreachable!(),
		InfixOp::Plus =>
			match rhs {
				Some(r_val) =>
					match lhs {
						// case a + b
						Some(l_val) => l_val.add(r_val),
						// case + a
						None => Ok(r_val)
					},
				None =>
					Err(RollerErr::EvalError(
						EvalErr::MissingOpArg{op: op, side: Side::Right}
					))
			},
		InfixOp::Minus =>
			match rhs {
				Some(r_val) =>
					match lhs {
						// case a - b
						Some(l_val) => l_val.sub(r_val),
						// case - a
						None => r_val.neg()
					},
				None =>
					Err(RollerErr::EvalError(
						EvalErr::MissingOpArg{op: op, side: Side::Right}
					))
			},
		InfixOp::Mul => expect_binary_op!(op, lhs, rhs, Value::mul),
		InfixOp::Div => expect_binary_op!(op, lhs, rhs, Value::div),
		InfixOp::Pow => expect_binary_op!(op, lhs, rhs, Value::pow),
	}
}
