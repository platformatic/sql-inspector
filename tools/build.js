// Copy the whole built file from `pkg` to root.

'use strict'

const { 
  copyFile, 
  // rm, 
  // access 
} = require('node:fs/promises')
const { join } = require('node:path')

async function copyFiles () {
  const filesToCopy = [
    'sql_inspector.js',
    'sql_inspector.d.ts',
    'sql_inspector_bg.wasm',
    'sql_inspector_bg.wasm.d.ts',
    'package.json'
  ]

  const pkgFolder = join(__dirname, '../pkg/')

  for (const file of filesToCopy) {
    const source = join(pkgFolder, file)
    const destination = join(__dirname, '..', file)
    await copyFile(source, destination)
  }

  console.log('Files copied successfully')

  // try {
  //   await rm(pkgFolder, { recursive: true })
  //   console.log('pkg folder deleted successfully')
  // } catch (err) {
  //   console.error('pkg folder not found')
  //   return
  // }
}
copyFiles()
