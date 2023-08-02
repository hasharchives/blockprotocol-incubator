# Blok

Blok (pronounced blɔk) is a simple language to express types of the Block Protocol, the language is a subset of
Typescript and allows to express the types of the protocol in a simple way.

Blok comes with several commands, these include:

* `diff`: Compare types between remote and local. This is useful to see what changes are required to sync types
* `sync`: Import/sync types from a remote system
  supports overrides, which are important to allow for easy local development
* `plan`: Plan execution of operations required to sync types to a remote system
* `apply`: Apply operations to sync types to a remote system

`diff` is to `sync`, what `plan` is to `apply`. `diff` and `plan` are read-only operations, while `sync` and `apply` are
read-write operations.
