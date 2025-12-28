//! Console-related tests

use super::eval;
use tsrun::JsValue;

#[test]
fn test_console_log() {
    assert_eq!(eval("console.log('test')"), JsValue::Undefined);
}

#[test]
fn test_console_error() {
    assert_eq!(eval("console.error('test')"), JsValue::Undefined);
}

#[test]
fn test_console_warn() {
    assert_eq!(eval("console.warn('test')"), JsValue::Undefined);
}

#[test]
fn test_console_info() {
    assert_eq!(eval("console.info('test')"), JsValue::Undefined);
}

#[test]
fn test_console_debug() {
    assert_eq!(eval("console.debug('test')"), JsValue::Undefined);
}

#[test]
fn test_console_table() {
    // table returns undefined
    assert_eq!(eval("console.table([1, 2, 3])"), JsValue::Undefined);
}

#[test]
fn test_console_dir() {
    // dir returns undefined
    assert_eq!(eval("console.dir({a: 1})"), JsValue::Undefined);
}

#[test]
fn test_console_time() {
    // time returns undefined
    assert_eq!(eval("console.time('test')"), JsValue::Undefined);
}

#[test]
fn test_console_time_end() {
    // timeEnd returns undefined
    assert_eq!(
        eval("console.time('test'); console.timeEnd('test')"),
        JsValue::Undefined
    );
}

#[test]
fn test_console_count() {
    // count returns undefined
    assert_eq!(eval("console.count('test')"), JsValue::Undefined);
}

#[test]
fn test_console_count_reset() {
    // countReset returns undefined
    assert_eq!(eval("console.countReset('test')"), JsValue::Undefined);
}

#[test]
fn test_console_clear() {
    // clear returns undefined
    assert_eq!(eval("console.clear()"), JsValue::Undefined);
}

#[test]
fn test_console_group() {
    // group returns undefined
    assert_eq!(eval("console.group('test')"), JsValue::Undefined);
}

#[test]
fn test_console_group_end() {
    // groupEnd returns undefined
    assert_eq!(eval("console.groupEnd()"), JsValue::Undefined);
}
