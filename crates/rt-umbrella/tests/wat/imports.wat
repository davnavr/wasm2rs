(module
  (import "tests" "memory" (memory 1))
  (import "tests" "FORTY" (global $forty i32))
  (import "tests" "assert_equal" (func $assert_equal (param i32 i32)))

  (func (export "funny_life_number") (result i32)
    global.get $forty
    i32.const 2
    i32.add)

  (func (export "two_equals_two")
    i32.const 2
    i32.const 2
    call $assert_equal)

  (func (export "write_5_to_5000")
    i32.const 5000
    i32.const 5
    i32.store)
)
