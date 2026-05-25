#!/usr/bin/env bash
# schema-ref.sh — Generate a consolidated schema reference for LLM/human use.
#
# Usage:
#   ./schema-ref.sh [DATABASE_URL] [OUTPUT_FILE] [SCHEMA]
#
# Defaults:
#   DATABASE_URL  $DATABASE_URL env var, or postgresql://postgres:secret@localhost/hsaportal_dev
#   OUTPUT_FILE   schema-ref.txt  (use "-" for stdout)
#   SCHEMA        public

set -euo pipefail

DB="${1:-${DATABASE_URL:-postgresql://postgres:secret@localhost/hsaportal_dev}}"
OUTPUT="${2:-schema-ref.txt}"
SCHEMA="${3:-public}"

# Run a query, return tuples only, unaligned
q() { psql -t -A -d "$DB" -c "$1"; }

rule80()  { printf '=%.0s' {1..80}; echo; }
rule80s() { printf -- '-%.0s' {1..80}; echo; }

indent_comment() {
    # Indent a multiline comment string, stripping leading/trailing blank lines
    local prefix="${1}"
    local text="${2}"
    echo "$text" \
        | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' \
        | grep -v '^$' \
        | sed "s/^/${prefix}/"
}

generate() {

# ---------------------------------------------------------------------------
# DOMAINS
# ---------------------------------------------------------------------------
rule80
echo "DOMAINS"
rule80

q "
SELECT format(E'%s\n  Base type : %s\n  Not null  : %s\n  Default   : %s\n  Check     : %s\n  Comment   : %s\n',
    t.typname,
    pg_catalog.format_type(t.typbasetype, t.typtypmod),
    CASE WHEN t.typnotnull THEN 'yes' ELSE 'no' END,
    COALESCE(t.typdefault, '(none)'),
    COALESCE(
        (SELECT string_agg(pg_get_constraintdef(c.oid, true), ', ')
         FROM pg_constraint c WHERE c.contypid = t.oid),
        '(none)'
    ),
    COALESCE(regexp_replace(trim(obj_description(t.oid, 'pg_type')), '\s+\$', '', 'gm'), '(none)')
)
FROM pg_type t
JOIN pg_namespace n ON n.oid = t.typnamespace AND n.nspname = '$SCHEMA'
WHERE t.typtype = 'd'
ORDER BY t.typname
"

# ---------------------------------------------------------------------------
# ENUMS
# ---------------------------------------------------------------------------
rule80
echo "ENUMS"
rule80

q "
SELECT format(E'%s\n  Values  : %s\n  Comment : %s\n',
    t.typname,
    string_agg(e.enumlabel, ', ' ORDER BY e.enumsortorder),
    COALESCE(trim(obj_description(t.oid, 'pg_type')), '(none)')
)
FROM pg_type t
JOIN pg_namespace n ON n.oid = t.typnamespace AND n.nspname = '$SCHEMA'
JOIN pg_enum e ON e.enumtypid = t.oid
GROUP BY t.oid, t.typname
ORDER BY t.typname
"

# ---------------------------------------------------------------------------
# TABLES
# ---------------------------------------------------------------------------
rule80
echo "TABLES"
rule80

TABLES=$(q "
    SELECT c.relname
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = '$SCHEMA'
    WHERE c.relkind = 'r'
    ORDER BY c.relname
")

while IFS= read -r table; do
    [[ -z "$table" ]] && continue

    echo
    rule80s
    echo "TABLE: $SCHEMA.$table"
    rule80s

    # Table comment
    TABLE_COMMENT=$(q "SELECT COALESCE(trim(obj_description('$SCHEMA.$table'::regclass, 'pg_class')), '')")
    if [[ -n "$TABLE_COMMENT" ]]; then
        indent_comment "  " "$TABLE_COMMENT"
    fi

    # Columns
    echo
    echo "  Columns:"
    q "
    SELECT format('    %-28s  %-38s  %s%s',
        a.attname,
        pg_catalog.format_type(a.atttypid, a.atttypmod)
            || CASE WHEN a.attidentity != '' THEN ' [identity]' ELSE '' END
            || CASE WHEN a.attgenerated != '' THEN ' [generated]' ELSE '' END,
        CASE WHEN a.attnotnull THEN 'NOT NULL' ELSE 'NULL    ' END,
        CASE WHEN d.adbin IS NOT NULL AND a.attidentity = '' AND a.attgenerated = ''
             THEN '  DEFAULT ' || pg_get_expr(d.adbin, d.adrelid)
             ELSE ''
        END
    )
    FROM pg_attribute a
    LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
    WHERE a.attrelid = '$SCHEMA.$table'::regclass
      AND a.attnum > 0
      AND NOT a.attisdropped
    ORDER BY a.attnum
    "

    # Column comments (only those that have them)
    COL_COMMENTS=$(q "
    SELECT format('    %-28s  %s',
        a.attname,
        replace(
            regexp_replace(trim(col_description(a.attrelid, a.attnum)), '\s+\$', '', 'gm'),
            E'\n',
            E'\n' || repeat(' ', 32)
        )
    )
    FROM pg_attribute a
    WHERE a.attrelid = '$SCHEMA.$table'::regclass
      AND a.attnum > 0
      AND NOT a.attisdropped
      AND col_description(a.attrelid, a.attnum) IS NOT NULL
    ORDER BY a.attnum
    ")

    if [[ -n "$COL_COMMENTS" ]]; then
        echo
        echo "  Column Comments:"
        echo "$COL_COMMENTS"
    fi

    # Constraints
    CONSTRAINTS=$(q "
    SELECT format('    [%s] %-52s  %s',
        CASE c.contype
            WHEN 'p' THEN 'PK'
            WHEN 'f' THEN 'FK'
            WHEN 'u' THEN 'UQ'
            WHEN 'c' THEN 'CK'
            ELSE c.contype::text
        END,
        c.conname,
        pg_get_constraintdef(c.oid, true)
    )
    FROM pg_constraint c
    WHERE c.conrelid = '$SCHEMA.$table'::regclass
    ORDER BY
        CASE c.contype WHEN 'p' THEN 0 WHEN 'u' THEN 1 WHEN 'f' THEN 2 WHEN 'c' THEN 3 ELSE 4 END,
        c.conname
    ")

    if [[ -n "$CONSTRAINTS" ]]; then
        echo
        echo "  Constraints:"
        echo "$CONSTRAINTS"
    fi

    # Non-constraint-backing indexes
    INDEXES=$(q "
    SELECT format('    %s', pg_get_indexdef(x.indexrelid))
    FROM pg_index x
    JOIN pg_class ic ON ic.oid = x.indexrelid
    WHERE x.indrelid = '$SCHEMA.$table'::regclass
      AND NOT EXISTS (
          SELECT 1 FROM pg_constraint c WHERE c.conindid = x.indexrelid
      )
    ORDER BY ic.relname
    ")

    if [[ -n "$INDEXES" ]]; then
        echo
        echo "  Indexes:"
        echo "$INDEXES"
    fi

    # Triggers
    TRIGGERS=$(q "
    SELECT format('    %-40s  %s',
        t.tgname,
        pg_get_triggerdef(t.oid, true)
    )
    FROM pg_trigger t
    WHERE t.tgrelid = '$SCHEMA.$table'::regclass
      AND NOT t.tgisinternal
    ORDER BY t.tgname
    ")

    if [[ -n "$TRIGGERS" ]]; then
        echo
        echo "  Triggers:"
        echo "$TRIGGERS"
    fi

done <<< "$TABLES"

# ---------------------------------------------------------------------------
# VIEWS
# ---------------------------------------------------------------------------
echo
rule80
echo "VIEWS"
rule80

VIEWS=$(q "
    SELECT c.relname
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = '$SCHEMA'
    WHERE c.relkind = 'v'
    ORDER BY c.relname
")

while IFS= read -r view; do
    [[ -z "$view" ]] && continue

    echo
    rule80s
    echo "VIEW: $SCHEMA.$view"
    rule80s

    VIEW_COMMENT=$(q "SELECT COALESCE(trim(obj_description('$SCHEMA.$view'::regclass, 'pg_class')), '')")
    if [[ -n "$VIEW_COMMENT" ]]; then
        indent_comment "  " "$VIEW_COMMENT"
        echo
    fi

    q "SELECT pg_get_viewdef('$SCHEMA.$view'::regclass, true)" | sed 's/^/  /'

done <<< "$VIEWS"

# ---------------------------------------------------------------------------
# FUNCTIONS
# ---------------------------------------------------------------------------
echo
rule80
echo "FUNCTIONS"
rule80
echo

q "
SELECT format('  %s(%s) -> %s%s',
    p.proname,
    pg_get_function_arguments(p.oid),
    pg_get_function_result(p.oid),
    CASE WHEN obj_description(p.oid, 'pg_proc') IS NOT NULL
         THEN E'\n    ' || replace(trim(obj_description(p.oid, 'pg_proc')), E'\n', E'\n    ')
         ELSE ''
    END
)
FROM pg_proc p
JOIN pg_namespace n ON n.oid = p.pronamespace AND n.nspname = '$SCHEMA'
WHERE p.prokind IN ('f', 'p')
ORDER BY p.proname, pg_get_function_arguments(p.oid)
"

} # end generate

if [[ "$OUTPUT" == "-" ]]; then
    generate
else
    generate > "$OUTPUT"
    echo "Schema reference written to: $OUTPUT" >&2
fi
