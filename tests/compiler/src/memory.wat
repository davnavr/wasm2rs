(module
  (memory 1)
  (func (export "write_my_int")
    i32.const 500
    i32.const 65
    i32.store)

  (func (export "read_my_int") (result i32)
    i32.const 500
    i32.load)

  (func (export "out_of_bounds_read") (result i32)
    i32.const 65536
    i32.load)

  (func (export "grow_then_write") (result i32)
    (local $old_size i32)
    (local.set $old_size (memory.size))
    (i32.mul (local.get $old_size) (i32.const 65536))

    ;; grow then write to this address
    ;; (memory.grow (i32.add (i32.const 1) (local.get $old_size)))

    ;; i32.const 0xBABA
    ;; i32.store

    ;; (i32.load (i32.const 0xBABA))
    )
)
