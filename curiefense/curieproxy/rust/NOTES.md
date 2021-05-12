# Current status

An initial implementation of all filtering components had been written. The following is currently handled in Lua:

 * getting the information required by the filtering engine:
    - 
 * request replies
 * logging

## Things to fix

 * implement asynchronous methods so as not to block envoy -> unfortunately much harder than expected, will probably need a custom executor to work properly
 * performance profiling
 * code structure / refactorings / etc.

# Main API

The main API can be used to perform filtering operations.

## Functions

### `inspect_request`

Takes five arguments:

 * *meta*, a Lua table containing the following entries:

    - `path`: the "path" part of the HTTP request, containing the full, raw URI (ie. something like `/a/b?c=d&e=f`);
    - `method`: the HTTP verb (such as `GET`, `POST`, etc.);
    - `authority`: optionnaly, the `:authority` HTTP2 header, if available.
    
 * *headers*, a Lua table containing the HTTP headers (keys are the header names, values the header values).
 * *body*, optionnaly, the HTTP request body. Note that large bodies will have a performance impact, and should be size-limited before calling this function.
 * *ip*, the string-encoded IP address in canonical format.
 * a Lua table containing the *grasshopper* functions (such as the imported grasshopper module), or `nil` if not available.

It will perform all the curieproxy checks, and return a pair, with:

 * a JSON-encoded Decision (see below),
 * a list of strings, containing all encountered errors

# Session API

The session API can be used for fine grained control over the matching process.

## Functions

All functions return pairs, where the first value is the function result (possibly `nil` for functions that don't return anything),
and the second value is a string encoded error (`nil` when there were no errors).

Most functions need other functions to be called before being available:

 * the `rust_init_config` **MUST** be called before any other function is called
 * the `rust_session_init` must be called before any function taking a `session_id` as an argument
 * once the `rust_session_clean` function is called, the corresponding `session_id` is invalidated and will not work anymore
 * the `rust_session_match_urlmap` must be called before most matching functions, as described in the following documentation

### `init_config`

Called without arguments.

Returns a boolean value, `true` meaning the config was loaded without errors.

The "error" part of the returned pair is a list of strings, and not a single string, when errors happen.
It will list all problems encountered when loading the configuration files.

### `session_init`

Takes a single argument : JSON-encoded string representing the *request_map*.

Returns a string, representing a *session id*.

### `session_clean`

Takes a single argument: the *session id*.

Returns a value that can be discarded.

There must be a single call to `session_clean` for each call to `session_init` in order to prevent memory leaks.

### `session_serialize_request_map`

Takes a single argument: the *session id*.

Returns a JSON-encoded object, which is identical to the object sent to `session_init`, except for the list of tags which could have been updated as a result of calling any of the other functions.

### `session_match_urlmap`

Takes a single argument: the *session id*.

Returns a JSON-encoded object, that looks like:

```json
{
   "acl_profile" : "34511ea458ac",
   "acl_active" : true,
   "urlmap" : "default entry",
   "name" : "admin path",
   "waf_profile" : "__default__",
   "limit_ids" : [],
   "waf_active" : true
}
```

It has the same format as a configuration *urlmap entry*, except:

 * there is no `match` field
 * the `urlmap` field contains the name of the matched *urlmap*

This function updates the tags with the urlmap specific tags.

### `session_tag_request`

Takes a single argument: the *session id*.

Returns a value that can be discarded.

### `session_flow_check`

Takes a single argument: the *session id*.

Returns a decision (see below).

### `session_limit_check`

**`session_match_urlmap` must have been called before using this function!**

Takes a single argument: the *session id*.

Returns a decision (see below).

### `session_acl_check`

**`session_match_urlmap` must have been called before using this function!**

Takes a single argument: the *session id*.

On success, returns a JSON encoded object, having a single key:

 * if the key is `Bypass`, it represents a force deny/bypass decision
 * if the key is `Match`, it represents the decisions for humans and bots

In all cases, the matching tags are collected. Examples:

```json
{"Match":{"human":null,"bot":null}}
```

No match has been found (results in filtering proceeding to WAF checks).

```json
{"Match":{"human":{"tags":["foo"],"allowed":true},"bot":{"tags":["bar"],"allowed":false}}}
```
Humans are allowed, bots are denied (results in humans being accepted, and bots being challenged).

```json
{"Match":{"human":{"tags":["yyy"],"allowed":false},"bot":null}}
```

Humans are denied, bots are not matched (results in a deny).

```json
{"Bypass":{"tags":["xxx"],"allowed":true}}
```

Bypass (results in the request being allowed).

```json
{"Bypass":{"tags":["xxx"],"allowed":false}}
```

Force deny (results in the request being dropped).

### `session_waf_check`

**`session_match_urlmap` must have been called before using this function!**

Takes a single argument: the *session id*.

Returns a decision (see below).

### The decision data structure

The decision is a json encoded value, with can be of the following form:

 * The string `"Pass"`, meaning the request is allowed at this stage,
 * An object with a single key, `Action`, and a single value representing the action to be taken.

Example, when actions needs to be taken:

```json
{
   "action" : "custom_response",
   "logs" : [
      {
         "elapsed_micros" : 20,
         "level" : "debug",
         "message" : "body parsing start"
      },
      {
         "elapsed_micros" : 23,
         "level" : "debug",
         "message" : "parsing content type: multipart/form-data; boundary=------------------------07502956c60abf2d"
      },
      {
         "elapsed_micros" : 67,
         "level" : "debug",
         "message" : "Selected hostmap default entry"
      },
      {
         "elapsed_micros" : 70,
         "level" : "debug",
         "message" : "Selected hostmap entry test path"
      }
   ],
   "request_map" : {
      "args" : {
         "a" : "b",
         "baz" : "qux",
         "c" : "cmd.exe",
         "foo" : "bar"
      },
      "attrs" : {
         "ip" : "172.19.0.1",
         "ipnum" : "2886926337",
         "path" : "/test/",
         "query" : "a=b&c=cmd.exe",
         "remote_addr" : "172.19.0.1",
         "uri" : "/test/?a=b&c=cmd.exe"
      },
      "cookies" : {},
      "geo" : {
         "city" : {},
         "continent" : {},
         "country" : {},
         "location" : {}
      },
      "headers" : {
         "accept" : "*/*",
         "content-length" : "236",
         "content-type" : "multipart/form-data; boundary=------------------------07502956c60abf2d",
         "user-agent" : "curl/7.68.0",
         "x-envoy-internal" : "true",
         "x-forwarded-for" : "172.19.0.1",
         "x-forwarded-proto" : "http",
         "x-request-id" : "e56d8114-df7e-4e9d-b0c0-2ac9cf302118"
      },
      "tags" : [
         "urlmap-entry:test-path",
         "container:a84fe3412aab",
         "aclname:demo-acl",
         "api",
         "aclid:34511ea458a8",
         "wafid:default-waf",
         "ip:172-19-0-1",
         "asn:nil",
         "urlmap:default-entry",
         "geo:nil",
         "all"
      ]
   },
   "response" : {
      "atype" : "block",
      "ban" : false,
      "block_mode" : true,
      "content" : "Access denied",
      "extra_tags" : null,
      "headers" : null,
      "reason" : {
         "initiator" : "waf",
         "name" : "content-type",
         "section" : "headers",
         "sig_category" : "sqli",
         "sig_id" : "100031",
         "sig_msg" : "SQLi Attempt (Escape Technique Captured)",
         "sig_operand" : "(\"'|'\"|--)[\\s\\<\\>;-]",
         "sig_severity" : 5,
         "sig_subcategory" : "escape-character",
         "value" : "multipart/form-data; boundary=------------------------07502956c60abf2d"
      },
      "status" : 403
   }
}
```

The fields have the following meaning:

 * `action`: can be either `pass` or `custom_response` ;
 * `response`: set when in `custom_response` mode, contains the data that is necessary for logging the reason a request was blocked (or flagged by an inactive WAF/ACL checker) ;
 * `request_map`: the request map, for logging purposes ;
 * `logs`: contains a list of logs generated by the Rust code.

# Misc notes

## Arguments, cookies, headers collisions

The same header, or argument can appear multiple times in an HTTP request. For example, the following URI might be used:

```
/path?a=1&a=2
```

Other situations are:

  * multiple headers have the same name ;
  * multiple cookies have the same name ;
  * URI parameters clash with each other ;
  * URI parameters clash with body parameters ;
  * body parameters clash with each other.

When this happens, values are concatenated with a space separator. In the previous example, we would end up with the `a` parameter being equal to `1 2`.

## Body parsing behavior

Body parsing uses the body that is passed by calling code, as if it was a binary buffer.
The `Content-Type` header is checked, and the following decisions are made:

  * if it starts with `multipart/form-data; boundary=`, try multipart form-data decoding ;
  * if it ends with `/json`, try JSON decoding ;
  * if it ends with `/xml`, try XML decoding ;
  * if it is equal to `application/x-www-form-urlencoded`, try form-encoded decoding ;
  * if it is absent, or none of the previous tests were successful, try to decode as JSON, and, if it fails, as form-encoded.

If all failed, the body is decoded as an UTF8 string (using invalid codepoints where decoding failed), and made available as the `RAW_BODY` argument.

Multipart form-data and form-encoded formats do not have a special logic, as they encode simple key/values associations.

### JSON body parsing

JSON values are not simple key/values associations. For these reasons, scalar values anywhere in the JSON value are associated with argument names that represent the "path" to these values. Here are some examples:

```json
{"a": [true,null,{"z": 0.2}], "c": {"d": 12}}
```

Will encode as:

```
a_0=true
a_1=null
a_2_z=0.2
c_d=12
```

"Path" to the scalar values are made of the key values for objects, and indices for arrays, joined by the `_` character.
When there is no path (the JSON payload is a scalar), the path is set to `JSON_ROOT`.

Runtime performance of the JSON body parser might be negatively impacted by the size of the document, but also by its structure. Structures like:

```json
{"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA":[1,2,3,4,5,6]}
```

Will produce the following arguments:
```
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_1=1
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_2=2
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_3=3
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_4=4
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_5=5
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA_6=6
```

Memory usage can grow up quadratically with the original document size.

### XML body parsing

The XML code assumes the text encoding is UTF8, and will fail for other encodings.
Just as the JSON encoding, there is a notion of "path" to a value.
XML is however more complicated, as there are two kinds of values: text between elements, and parameters within elements.

Pathes are built by concatenating the local names of elements and the index of the position the element appears within an element. For example, the following document:

```xml
<a> a <b foo="bar">xxx</b> z </a>
```

Will be decoded as:

```
a1=a
a2bfoo=bar
a2b1=xxx
a3=z
```

This example highlights how the relative ordering of elements is encoded in the argument name.
There are however several subtleties.

First of all, text elements are trimmed for spaces, and are not stored as an argument when they are empty, with the exception of of leaf elements.
Indeed, without this exception, a document like `<a><b><c/></b></a>` would not produce any argument.
Here, `<c/>` is s leaf element, and the `a1b1c1` will be associated with the empty strings.

`CDATA` directives are not trimmed for spaces.

XML entities have a special treatment. They are stored with argument names of the form `_XMLENTITY_[TYPE]_[NAME]`,
where type can be `VALUE` for entity values, `SYSTEMID` for external system id entities, and `PUBLICID` for external public id entities.

The same memory problems that are present in the JSON parser. Another potential problem comes with matching rules for XML documents. As the index of elements is encoded, most rules will be of the type "regex" for arguments names, resulting in linear scanning of the arguments list.