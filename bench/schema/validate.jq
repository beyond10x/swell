# Validates one result file against the JSON Schema bench/schema/swell-bench-1.json, passed as
# $schema (jq --slurpfile schema ...), and prints one line per error, "<path>: <error>"; prints
# nothing for a valid file. The keywords read are the ones that schema uses: type, const, enum,
# pattern, minLength, minimum, minItems, required, properties, items, allOf, if/then/else and a
# $ref into #/$defs. A schema keyword outside that list is reported, so the schema cannot grow a
# rule that is not checked. Then the checks a JSON Schema cannot state: a measured row's median
# lies between the smallest and the largest of its values, a measured row that is not an
# aggregate has one value per run, and every row's min and max are those of its values.

def known: ["$schema", "$id", "$defs", "$ref", "title", "description", "type", "const", "enum",
  "pattern", "minLength", "minimum", "minItems", "required", "properties", "items", "allOf",
  "if", "then", "else"];

def is_type($t):
  if $t == "integer" then type == "number" and . == floor
  else type == $t end;

def errors($root; $s; $p):
  . as $v
  | ( $s | keys[] | select(. as $k | known | index($k) | not)
      | "\($p): the schema uses the keyword \(.), which this check does not read" ),
    ( if $s | has("$ref") then
        ($s["$ref"] | sub("^#/\\$defs/"; "")) as $name
        | $v | errors($root; $root["$defs"][$name]; $p)
      else empty end ),
    ( if $s | has("type") then
        ([$s.type] | flatten) as $ts
        | if any($ts[]; . as $t | $v | is_type($t)) then empty
          else "\($p): is \($v | type), wanted \($ts | join(" or "))" end
      else empty end ),
    ( if ($s | has("const")) and $v != $s.const then
        "\($p): is \($v | tojson), wanted \($s.const | tojson)" else empty end ),
    ( if ($s | has("enum")) and (any($s.enum[]; . == $v) | not) then
        "\($p): is \($v | tojson), not one of \($s.enum | tojson)" else empty end ),
    ( if ($s | has("pattern")) and ($v | type) == "string" and ($v | test($s.pattern) | not) then
        "\($p): \($v | tojson) does not match \($s.pattern)" else empty end ),
    ( if ($s | has("minLength")) and ($v | type) == "string" and ($v | length) < $s.minLength then
        "\($p): is shorter than \($s.minLength)" else empty end ),
    ( if ($s | has("minimum")) and ($v | type) == "number" and $v < $s.minimum then
        "\($p): \($v) is below \($s.minimum)" else empty end ),
    ( if ($s | has("minItems")) and ($v | type) == "array" and ($v | length) < $s.minItems then
        "\($p): has \($v | length) item(s), fewer than \($s.minItems)" else empty end ),
    ( if ($s | has("required")) and ($v | type) == "object" then
        $s.required[] | select(. as $k | $v | has($k) | not) | "\($p): missing \(.)"
      else empty end ),
    ( if ($s | has("properties")) and ($v | type) == "object" then
        $s.properties | to_entries[] | select(.key as $k | $v | has($k))
        | . as $e | $v[$e.key] | errors($root; $e.value; "\($p).\($e.key)")
      else empty end ),
    ( if ($s | has("items")) and ($v | type) == "array" then
        range(0; $v | length) as $i | $v[$i] | errors($root; $s.items; "\($p)[\($i)]")
      else empty end ),
    ( if $s | has("allOf") then $s.allOf[] as $sub | $v | errors($root; $sub; $p) else empty end ),
    ( if $s | has("if") then
        if ([$v | errors($root; $s.if; $p)] | length) == 0 then
          (if $s | has("then") then $v | errors($root; $s.then; $p) else empty end)
        else
          (if $s | has("else") then $v | errors($root; $s.else; $p) else empty end)
        end
      else empty end );

def semantic:
  (.rows // []) | to_entries[]
  | .key as $i | .value as $r | "$.rows[\($i)]" as $p
  | select(($r | type) == "object" and ($r.values | type) == "array")
  | ( if $r.status == "measured" and ($r.median | type) == "number" and ($r.values | length) > 0
         and ($r.median < ($r.values | min) or $r.median > ($r.values | max)) then
        "\($p): median \($r.median) lies outside its values" else empty end ),
    ( if $r.status == "measured" and ($r | has("aggregate") | not) and ($r.runs != ($r.values | length)) then
        "\($p): runs \($r.runs), but \($r.values | length) value(s)" else empty end ),
    ( if ($r | has("min")) and ($r.values | length) > 0 and $r.min != ($r.values | min) then
        "\($p): min \($r.min) is not the smallest value" else empty end ),
    ( if ($r | has("max")) and ($r.values | length) > 0 and $r.max != ($r.values | max) then
        "\($p): max \($r.max) is not the largest value" else empty end );

($schema[0]) as $root
| (errors($root; $root; "$"), semantic)
