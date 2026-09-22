#!/bin/bash

###########################################################################################
# Generate a badge.json on the root of the repo with the percentage of the exercises done #
# if it fail to run the badge contains nd %                                               #
# The badge json has this schema:                                                         #
# ```json                                                                                 #
# {                                                                                       #
#  "label": "Progress",                                                                   #
#  "message": "42%",                                                                      #
#  "schemaVersion": 1,                                                                    #
#  "color": "blue"                                                                        #
# }                                                                                       #
# ```                                                                                     #
# It generate also a last_ex_done.json with the last exercise done, 'nd' otherwise.       #
# The json schema for this is:                                                            #
# ```json                                                                                 #
# {                                                                                       #
#  "label": "Last exercise done                                                           #
#  "message": "CHAPTER|EXERCISE_NAME",                                                    #
#  "schemaVersion": 1,                                                                    #
#  "color": "orange"                                                                      #
# }                                                                                       #
# ```                                                                                     #
###########################################################################################

# define error badge.json
WRK_BASE_DIR=$(git rev-parse --show-toplevel)
EX_DIR="$WRK_BASE_DIR/exercises"
SOLVED_DB="$EX_DIR/progress.db"
TOTAL_EX=$( find "$EX_DIR" -name Cargo.toml | wc -l )
TOTAL_EX=${TOTAL_EX//[[:space:]]/}

BADGE_JSON_TEMPLATE='{"schemaVersion":1,"label":"Progress","message":"__VALUE__%","color":"blue", "style":"for-the-badge"}'
LAST_EX_DONE_JSON_TEMPLATE='{"schemaVersion":1,"label":"Last exercise done","message":"__VALUE__","color":"orange", "style":"for-the-badge"}'

function calc_perc() {
    if [[ -f $SOLVED_DB && ! -z "$(command -v sqlite3)" && $TOTAL_EX =~ ^[0-9]+$ && (( TOTAL_EX -gt 0)) ]]; then
        total_solved=$( sqlite3 "$SOLVED_DB" "select count(*) from open_exercises where solved=1;" )
        perc=$(( total_solved * 100 / TOTAL_EX ))
        echo "$perc"
    else
        echo "ND"
    fi
}

function calc_last_ex_done() {
    if [[ -f $SOLVED_DB && ! -z "$(command -v sqlite3)" ]]; then
        last_ex_done=$( sqlite3 "$SOLVED_DB" "SELECT chapter, exercise FROM open_exercises where solved=1 order by chapter desc, exercise desc limit 1;" )
        echo "$last_ex_done"
    else
        echo "ND"
    fi
}

# check presence of progress.db on exercises folder
PERC_CALC=$(calc_perc);
printf '%s\n' "${BADGE_JSON_TEMPLATE//__VALUE__/$PERC_CALC}" > "${WRK_BASE_DIR}"/badge.json

LAST_EX_DONE=$(calc_last_ex_done);
printf '%s\n' "${LAST_EX_DONE_JSON_TEMPLATE//__VALUE__/$LAST_EX_DONE}" > "${WRK_BASE_DIR}"/last_ex_done.json
