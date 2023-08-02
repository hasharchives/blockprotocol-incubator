# Blok

Blok (pronounced blɔk) is a simple language to express types of the Block Protocol, the language is a subset of
Typescript and allows to express the types of the protocol in a simple way.

Blok comes with several commands, these include:

* `diff`: Compare types between remote and local. This is useful to see what changes are required to sync types
* `sync`: Import/sync types from a remote system
  supports overrides, which are important to allow for easy local development
* `plan`: Plan execution of operations required to sync types to a remote system
* `apply`: Apply operations to sync types to a remote system
* `new`: Create a new property/entity/data type from a template
* `dump`: Dump all the types in a project to stdout

`diff` is to `sync`, what `plan` is to `apply`. `diff` and `plan` are read-only operations, while `sync` and `apply` are
read-write operations.

## Organisation

Files in a directory can be organised in whatever way you want, but to be able to use the `sync` command, the files need
to follow the tree structure depending of the scheme.

Currently the following schemes are supported:

* `bp`: Block Protocol
* `flat`: Flat structure

The scheme is chosen depending on the structure of the URL, if no scheme was provided, the `flat` scheme is used.

To recognise the scheme, every parent folder may have a `blok.json` file, which defines the scheme used, when resolving
the URL for a type, the scheme is determined by traversing the tree upwards until a `blokconfig.json5` file is found and
any `blok.json` file before that is deep-merged.

### Block Protocol

Urls like: `https://blockprotocol.org/@examples/types/entity-type/person/v/1` conform to the Block Protocol scheme, and
will be split into the following structure:

```
blockprotocol.org/
  examples/
    entities/
      person.blok
```

### Flat

Urls that do not follow any scheme are considered to be flat, and will be put into a folder corresponding to the domain
of the URL:

```
blockprotocol.org/
  xyz.blok
```

Every type under the domain **must** have a `@url` property, which is used to determine the URL of the type, the name of
the file will be the `title` of the type converted to `pascaleCase`.

## `blok.json` / `blok.json5`

This is used to configure the scheme of the directory and looks like this:

```json5
{
  scheme: 'bp',
  // or
  scheme: 'flat',
}
```

if the `scheme` is `bp`, additionally the `bp` keyword is supported, which allows the configuration of different
components of a Block Protocol style URL:

```json5
{
  scheme: 'bp',
  bp: {
    domain: 'blockprotocol.org',
    namespace: 'examples',
    type: 'entities',
  }
}
```

if the `scheme` is `flat`, every type under it **must** have a `@url` property, which is used to determine the URL of
the type.

## `blokconfig.json` / `blokconfig.json5`

This file **must** be at the root and is used to configure everything like linting to the remote system, environment
variables can be inserted using the `${NAME}` syntax.

```json5
{
  remote: {
    url: 'https://blockprotocol.org',
    auth: {
      type: 'kratos',
      username: '${KRATOS_USERNAME}',
      password: '${KRATOS_PASSWORD}',
    }
  },
  lint: {
    case: {
      id: 'kebab-case',
    }
  }
}
```
