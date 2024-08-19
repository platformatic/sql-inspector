const { test } = require('node:test')
const { deepEqual } = require('node:assert')
const { sqlinspector } = require('../pkg/sql_inspector')

// Minimal JS - WASM interop tests. The proper unit tests
// for the inspector are in Rust.
test('simple selects', async () => {
  const res = sqlinspector('select name, id from users;')
  const expected = {
    columns: ['id', 'name'],
    tables: ['users'],
    query_type: 'SELECT',
    target_table: ''
  }
  deepEqual(res, expected)

  {
    const res = sqlinspector('select name, id from users where age > 30;')
    const expected = {
      columns: ['age', 'id', 'name'],
      tables: ['users'],
      query_type: 'SELECT',
      target_table: ''
    }
    deepEqual(res, expected)
  }


  {
    const res = sqlinspector('select * from users u')
    const expected = {
      columns: ['*'],
      tables: ['users'],
      query_type: 'SELECT',
      target_table: ''
    }
    deepEqual(res, expected)
  }
})

test('simple insert', async () => {
  const res = sqlinspector("INSERT INTO users (id, name) VALUES (1, 'John')")
  const expected = {
    columns: ['users.id', 'users.name'],
    tables: ['users'],
    query_type: 'INSERT',
    target_table: 'users'
  }
  deepEqual(res, expected)
})

test('simple update', async () => {
  const res = sqlinspector('UPDATE users SET age = 30')
  const expected = {
    columns: ['users.age'],
    tables: ['users'],
    query_type: 'UPDATE',
    target_table: 'users'
  }
  deepEqual(res, expected)
})

test('simple delete', async () => {
  const res = sqlinspector('DELETE users WHERE age > 30')
  const expected = {
    columns: ['age'],
    tables: ['users'],
    query_type: 'DELETE',
    target_table: ''
  }
  deepEqual(res, expected)
})
