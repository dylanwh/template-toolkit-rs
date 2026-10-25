// Auto-generated from Template::Grammar (v3.009)

// Grammar symbols are interned as small integer ids rather than stored as
// `&'static str`: string pointers in these tables needed ~26k load-time
// relocations, dirtying ~0.5 MiB of the binary in every process.

/// Terminal (token type) id; see [`terminal_id`].
pub type TerminalId = u8;
/// Nonterminal (rule left-hand side) id; the rule comments name them.
pub type NonterminalId = u8;

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub actions: Option<&'static [(TerminalId, i16)]>,
    pub default: Option<i16>,
    pub gotos: &'static [(NonterminalId, u16)],
}

#[derive(Debug, Clone, Copy)]
pub struct RuleInfo {
    pub lhs: NonterminalId,
    pub len: usize,
    pub has_code: bool,
}

/// The id of a token type (`"IDENT"`, `";"`, `""` for end of input), or
/// `None` if the grammar never expects it.
pub fn terminal_id(name: &str) -> Option<TerminalId> {
    match name {
        "" => Some(0),
        "$" => Some(1),
        "${" => Some(2),
        "(" => Some(3),
        ")" => Some(4),
        "+" => Some(5),
        "/" => Some(6),
        ":" => Some(7),
        ";" => Some(8),
        "?" => Some(9),
        "AND" => Some(10),
        "ASSIGN" => Some(11),
        "BINOP" => Some(12),
        "BLOCK" => Some(13),
        "CALL" => Some(14),
        "CASE" => Some(15),
        "CAT" => Some(16),
        "CATCH" => Some(17),
        "CLEAR" => Some(18),
        "CMPOP" => Some(19),
        "COMMA" => Some(20),
        "DEBUG" => Some(21),
        "DEFAULT" => Some(22),
        "DIV" => Some(23),
        "DOT" => Some(24),
        "ELSE" => Some(25),
        "ELSIF" => Some(26),
        "END" => Some(27),
        "FILENAME" => Some(28),
        "FILTER" => Some(29),
        "FINAL" => Some(30),
        "FOR" => Some(31),
        "GET" => Some(32),
        "IDENT" => Some(33),
        "IF" => Some(34),
        "IN" => Some(35),
        "INCLUDE" => Some(36),
        "INSERT" => Some(37),
        "LAST" => Some(38),
        "LITERAL" => Some(39),
        "MACRO" => Some(40),
        "META" => Some(41),
        "MOD" => Some(42),
        "NEXT" => Some(43),
        "NOT" => Some(44),
        "NUMBER" => Some(45),
        "OR" => Some(46),
        "PERL" => Some(47),
        "PROCESS" => Some(48),
        "RAWPERL" => Some(49),
        "REF" => Some(50),
        "RETURN" => Some(51),
        "SET" => Some(52),
        "STOP" => Some(53),
        "SWITCH" => Some(54),
        "TEXT" => Some(55),
        "THROW" => Some(56),
        "TO" => Some(57),
        "TRY" => Some(58),
        "UNLESS" => Some(59),
        "USE" => Some(60),
        "VIEW" => Some(61),
        "WHILE" => Some(62),
        "WRAPPER" => Some(63),
        "[" => Some(64),
        "\"" => Some(65),
        "]" => Some(66),
        "{" => Some(67),
        "}" => Some(68),
        _ => None,
    }
}

pub fn lookup_lextable(token: &str) -> Option<&'static str> {
    match token {
        "!" => Some("NOT"),
        "!=" => Some("CMPOP"),
        "$" => Some("$"),
        "${" => Some("${"),
        "%" => Some("BINOP"),
        "&&" => Some("AND"),
        "(" => Some("("),
        ")" => Some(")"),
        "*" => Some("BINOP"),
        "+" => Some("+"),
        "," => Some("COMMA"),
        "-" => Some("BINOP"),
        "." => Some("DOT"),
        ".." => Some("TO"),
        "/" => Some("/"),
        ":" => Some(":"),
        ";" => Some(";"),
        "<" => Some("CMPOP"),
        "<=" => Some("CMPOP"),
        "=" => Some("ASSIGN"),
        "==" => Some("CMPOP"),
        "=>" => Some("ASSIGN"),
        ">" => Some("CMPOP"),
        ">=" => Some("CMPOP"),
        "?" => Some("?"),
        "AND" => Some("AND"),
        "BLOCK" => Some("BLOCK"),
        "BREAK" => Some("LAST"),
        "CALL" => Some("CALL"),
        "CASE" => Some("CASE"),
        "CATCH" => Some("CATCH"),
        "CLEAR" => Some("CLEAR"),
        "DEBUG" => Some("DEBUG"),
        "DEFAULT" => Some("DEFAULT"),
        "DIV" => Some("DIV"),
        "ELSE" => Some("ELSE"),
        "ELSIF" => Some("ELSIF"),
        "END" => Some("END"),
        "FILTER" => Some("FILTER"),
        "FINAL" => Some("FINAL"),
        "FOR" => Some("FOR"),
        "FOREACH" => Some("FOR"),
        "GET" => Some("GET"),
        "IF" => Some("IF"),
        "IN" => Some("IN"),
        "INCLUDE" => Some("INCLUDE"),
        "INSERT" => Some("INSERT"),
        "LAST" => Some("LAST"),
        "MACRO" => Some("MACRO"),
        "META" => Some("META"),
        "MOD" => Some("MOD"),
        "NEXT" => Some("NEXT"),
        "NOT" => Some("NOT"),
        "OR" => Some("OR"),
        "PERL" => Some("PERL"),
        "PLUGIN" => Some("PLUGIN"),
        "PROCESS" => Some("PROCESS"),
        "RAWPERL" => Some("RAWPERL"),
        "RETURN" => Some("RETURN"),
        "SET" => Some("SET"),
        "STEP" => Some("STEP"),
        "STOP" => Some("STOP"),
        "SWITCH" => Some("SWITCH"),
        "THROW" => Some("THROW"),
        "TO" => Some("TO"),
        "TRY" => Some("TRY"),
        "UNLESS" => Some("UNLESS"),
        "USE" => Some("USE"),
        "VIEW" => Some("VIEW"),
        "WHILE" => Some("WHILE"),
        "WRAPPER" => Some("WRAPPER"),
        "[" => Some("["),
        "\\" => Some("REF"),
        "]" => Some("]"),
        "_" => Some("CAT"),
        "and" => Some("AND"),
        "div" => Some("DIV"),
        "mod" => Some("MOD"),
        "not" => Some("NOT"),
        "or" => Some("OR"),
        "{" => Some("{"),
        "|" => Some("FILTER"),
        "||" => Some("OR"),
        "}" => Some("}"),
        _ => None,
    }
}

pub static RULES: &[RuleInfo] = &[
    // Rule 0
    RuleInfo {
        lhs: 0, /* $start */
        len: 2,
        has_code: false,
    },
    // Rule 1
    RuleInfo {
        lhs: 57, /* template */
        len: 1,
        has_code: true,
    },
    // Rule 2
    RuleInfo {
        lhs: 11, /* block */
        len: 1,
        has_code: true,
    },
    // Rule 3
    RuleInfo {
        lhs: 11, /* block */
        len: 0,
        has_code: true,
    },
    // Rule 4
    RuleInfo {
        lhs: 17, /* chunks */
        len: 2,
        has_code: true,
    },
    // Rule 5
    RuleInfo {
        lhs: 17, /* chunks */
        len: 1,
        has_code: true,
    },
    // Rule 6
    RuleInfo {
        lhs: 16, /* chunk */
        len: 1,
        has_code: true,
    },
    // Rule 7
    RuleInfo {
        lhs: 16, /* chunk */
        len: 2,
        has_code: true,
    },
    // Rule 8
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 9
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 10
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 11
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 12
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 13
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 14
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 15
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: false,
    },
    // Rule 16
    RuleInfo {
        lhs: 54, /* statement */
        len: 1,
        has_code: true,
    },
    // Rule 17
    RuleInfo {
        lhs: 54, /* statement */
        len: 2,
        has_code: true,
    },
    // Rule 18
    RuleInfo {
        lhs: 54, /* statement */
        len: 0,
        has_code: false,
    },
    // Rule 19
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: true,
    },
    // Rule 20
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 21
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 22
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 23
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 24
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 25
    RuleInfo {
        lhs: 21, /* directive */
        len: 1,
        has_code: false,
    },
    // Rule 26
    RuleInfo {
        lhs: 10, /* atomexpr */
        len: 1,
        has_code: true,
    },
    // Rule 27
    RuleInfo {
        lhs: 10, /* atomexpr */
        len: 1,
        has_code: false,
    },
    // Rule 28
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 29
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 30
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 31
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 32
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 33
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 34
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 35
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 36
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: true,
    },
    // Rule 37
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: true,
    },
    // Rule 38
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: true,
    },
    // Rule 39
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: true,
    },
    // Rule 40
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: true,
    },
    // Rule 41
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 2,
        has_code: true,
    },
    // Rule 42
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: false,
    },
    // Rule 43
    RuleInfo {
        lhs: 9, /* atomdir */
        len: 1,
        has_code: false,
    },
    // Rule 44
    RuleInfo {
        lhs: 18, /* condition */
        len: 6,
        has_code: true,
    },
    // Rule 45
    RuleInfo {
        lhs: 18, /* condition */
        len: 3,
        has_code: true,
    },
    // Rule 46
    RuleInfo {
        lhs: 18, /* condition */
        len: 6,
        has_code: true,
    },
    // Rule 47
    RuleInfo {
        lhs: 18, /* condition */
        len: 3,
        has_code: true,
    },
    // Rule 48
    RuleInfo {
        lhs: 22, /* else */
        len: 5,
        has_code: true,
    },
    // Rule 49
    RuleInfo {
        lhs: 22, /* else */
        len: 3,
        has_code: true,
    },
    // Rule 50
    RuleInfo {
        lhs: 22, /* else */
        len: 0,
        has_code: true,
    },
    // Rule 51
    RuleInfo {
        lhs: 56, /* switch */
        len: 6,
        has_code: true,
    },
    // Rule 52
    RuleInfo {
        lhs: 15, /* case */
        len: 5,
        has_code: true,
    },
    // Rule 53
    RuleInfo {
        lhs: 15, /* case */
        len: 4,
        has_code: true,
    },
    // Rule 54
    RuleInfo {
        lhs: 15, /* case */
        len: 3,
        has_code: true,
    },
    // Rule 55
    RuleInfo {
        lhs: 15, /* case */
        len: 0,
        has_code: true,
    },
    // Rule 56
    RuleInfo {
        lhs: 1, /* @1-3 */
        len: 0,
        has_code: true,
    },
    // Rule 57
    RuleInfo {
        lhs: 33, /* loop */
        len: 6,
        has_code: true,
    },
    // Rule 58
    RuleInfo {
        lhs: 33, /* loop */
        len: 3,
        has_code: true,
    },
    // Rule 59
    RuleInfo {
        lhs: 2, /* @2-3 */
        len: 0,
        has_code: true,
    },
    // Rule 60
    RuleInfo {
        lhs: 33, /* loop */
        len: 6,
        has_code: true,
    },
    // Rule 61
    RuleInfo {
        lhs: 33, /* loop */
        len: 3,
        has_code: true,
    },
    // Rule 62
    RuleInfo {
        lhs: 34, /* loopvar */
        len: 4,
        has_code: true,
    },
    // Rule 63
    RuleInfo {
        lhs: 34, /* loopvar */
        len: 4,
        has_code: true,
    },
    // Rule 64
    RuleInfo {
        lhs: 34, /* loopvar */
        len: 2,
        has_code: true,
    },
    // Rule 65
    RuleInfo {
        lhs: 62, /* wrapper */
        len: 5,
        has_code: true,
    },
    // Rule 66
    RuleInfo {
        lhs: 62, /* wrapper */
        len: 3,
        has_code: true,
    },
    // Rule 67
    RuleInfo {
        lhs: 59, /* try */
        len: 5,
        has_code: true,
    },
    // Rule 68
    RuleInfo {
        lhs: 27, /* final */
        len: 5,
        has_code: true,
    },
    // Rule 69
    RuleInfo {
        lhs: 27, /* final */
        len: 5,
        has_code: true,
    },
    // Rule 70
    RuleInfo {
        lhs: 27, /* final */
        len: 4,
        has_code: true,
    },
    // Rule 71
    RuleInfo {
        lhs: 27, /* final */
        len: 3,
        has_code: true,
    },
    // Rule 72
    RuleInfo {
        lhs: 27, /* final */
        len: 0,
        has_code: true,
    },
    // Rule 73
    RuleInfo {
        lhs: 60, /* use */
        len: 2,
        has_code: true,
    },
    // Rule 74
    RuleInfo {
        lhs: 3, /* @3-3 */
        len: 0,
        has_code: true,
    },
    // Rule 75
    RuleInfo {
        lhs: 61, /* view */
        len: 6,
        has_code: true,
    },
    // Rule 76
    RuleInfo {
        lhs: 4, /* @4-2 */
        len: 0,
        has_code: true,
    },
    // Rule 77
    RuleInfo {
        lhs: 48, /* perl */
        len: 5,
        has_code: true,
    },
    // Rule 78
    RuleInfo {
        lhs: 5, /* @5-1 */
        len: 0,
        has_code: true,
    },
    // Rule 79
    RuleInfo {
        lhs: 52, /* rawperl */
        len: 5,
        has_code: true,
    },
    // Rule 80
    RuleInfo {
        lhs: 26, /* filter */
        len: 5,
        has_code: true,
    },
    // Rule 81
    RuleInfo {
        lhs: 26, /* filter */
        len: 3,
        has_code: true,
    },
    // Rule 82
    RuleInfo {
        lhs: 19, /* defblock */
        len: 5,
        has_code: true,
    },
    // Rule 83
    RuleInfo {
        lhs: 20, /* defblockname */
        len: 2,
        has_code: true,
    },
    // Rule 84
    RuleInfo {
        lhs: 13, /* blockname */
        len: 1,
        has_code: false,
    },
    // Rule 85
    RuleInfo {
        lhs: 13, /* blockname */
        len: 1,
        has_code: true,
    },
    // Rule 86
    RuleInfo {
        lhs: 12, /* blockargs */
        len: 1,
        has_code: false,
    },
    // Rule 87
    RuleInfo {
        lhs: 12, /* blockargs */
        len: 0,
        has_code: false,
    },
    // Rule 88
    RuleInfo {
        lhs: 6, /* anonblock */
        len: 5,
        has_code: true,
    },
    // Rule 89
    RuleInfo {
        lhs: 14, /* capture */
        len: 3,
        has_code: true,
    },
    // Rule 90
    RuleInfo {
        lhs: 37, /* macro */
        len: 6,
        has_code: true,
    },
    // Rule 91
    RuleInfo {
        lhs: 37, /* macro */
        len: 3,
        has_code: true,
    },
    // Rule 92
    RuleInfo {
        lhs: 39, /* mdir */
        len: 1,
        has_code: false,
    },
    // Rule 93
    RuleInfo {
        lhs: 39, /* mdir */
        len: 4,
        has_code: true,
    },
    // Rule 94
    RuleInfo {
        lhs: 38, /* margs */
        len: 2,
        has_code: true,
    },
    // Rule 95
    RuleInfo {
        lhs: 38, /* margs */
        len: 2,
        has_code: true,
    },
    // Rule 96
    RuleInfo {
        lhs: 38, /* margs */
        len: 1,
        has_code: true,
    },
    // Rule 97
    RuleInfo {
        lhs: 41, /* metadata */
        len: 2,
        has_code: true,
    },
    // Rule 98
    RuleInfo {
        lhs: 41, /* metadata */
        len: 2,
        has_code: false,
    },
    // Rule 99
    RuleInfo {
        lhs: 41, /* metadata */
        len: 1,
        has_code: false,
    },
    // Rule 100
    RuleInfo {
        lhs: 40, /* meta */
        len: 3,
        has_code: true,
    },
    // Rule 101
    RuleInfo {
        lhs: 40, /* meta */
        len: 5,
        has_code: true,
    },
    // Rule 102
    RuleInfo {
        lhs: 40, /* meta */
        len: 3,
        has_code: true,
    },
    // Rule 103
    RuleInfo {
        lhs: 58, /* term */
        len: 1,
        has_code: false,
    },
    // Rule 104
    RuleInfo {
        lhs: 58, /* term */
        len: 1,
        has_code: false,
    },
    // Rule 105
    RuleInfo {
        lhs: 35, /* lterm */
        len: 3,
        has_code: true,
    },
    // Rule 106
    RuleInfo {
        lhs: 35, /* lterm */
        len: 3,
        has_code: true,
    },
    // Rule 107
    RuleInfo {
        lhs: 35, /* lterm */
        len: 2,
        has_code: true,
    },
    // Rule 108
    RuleInfo {
        lhs: 35, /* lterm */
        len: 3,
        has_code: true,
    },
    // Rule 109
    RuleInfo {
        lhs: 55, /* sterm */
        len: 1,
        has_code: true,
    },
    // Rule 110
    RuleInfo {
        lhs: 55, /* sterm */
        len: 2,
        has_code: true,
    },
    // Rule 111
    RuleInfo {
        lhs: 55, /* sterm */
        len: 3,
        has_code: true,
    },
    // Rule 112
    RuleInfo {
        lhs: 55, /* sterm */
        len: 1,
        has_code: false,
    },
    // Rule 113
    RuleInfo {
        lhs: 55, /* sterm */
        len: 1,
        has_code: false,
    },
    // Rule 114
    RuleInfo {
        lhs: 31, /* list */
        len: 2,
        has_code: true,
    },
    // Rule 115
    RuleInfo {
        lhs: 31, /* list */
        len: 2,
        has_code: false,
    },
    // Rule 116
    RuleInfo {
        lhs: 31, /* list */
        len: 1,
        has_code: false,
    },
    // Rule 117
    RuleInfo {
        lhs: 51, /* range */
        len: 3,
        has_code: true,
    },
    // Rule 118
    RuleInfo {
        lhs: 28, /* hash */
        len: 1,
        has_code: false,
    },
    // Rule 119
    RuleInfo {
        lhs: 28, /* hash */
        len: 0,
        has_code: true,
    },
    // Rule 120
    RuleInfo {
        lhs: 47, /* params */
        len: 2,
        has_code: true,
    },
    // Rule 121
    RuleInfo {
        lhs: 47, /* params */
        len: 2,
        has_code: false,
    },
    // Rule 122
    RuleInfo {
        lhs: 47, /* params */
        len: 1,
        has_code: false,
    },
    // Rule 123
    RuleInfo {
        lhs: 46, /* param */
        len: 3,
        has_code: true,
    },
    // Rule 124
    RuleInfo {
        lhs: 46, /* param */
        len: 3,
        has_code: true,
    },
    // Rule 125
    RuleInfo {
        lhs: 29, /* ident */
        len: 3,
        has_code: true,
    },
    // Rule 126
    RuleInfo {
        lhs: 29, /* ident */
        len: 3,
        has_code: true,
    },
    // Rule 127
    RuleInfo {
        lhs: 29, /* ident */
        len: 1,
        has_code: false,
    },
    // Rule 128
    RuleInfo {
        lhs: 45, /* node */
        len: 1,
        has_code: true,
    },
    // Rule 129
    RuleInfo {
        lhs: 45, /* node */
        len: 4,
        has_code: true,
    },
    // Rule 130
    RuleInfo {
        lhs: 30, /* item */
        len: 1,
        has_code: true,
    },
    // Rule 131
    RuleInfo {
        lhs: 30, /* item */
        len: 3,
        has_code: true,
    },
    // Rule 132
    RuleInfo {
        lhs: 30, /* item */
        len: 2,
        has_code: true,
    },
    // Rule 133
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 134
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 135
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 136
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 137
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 138
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 139
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 140
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 141
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 142
    RuleInfo {
        lhs: 23, /* expr */
        len: 2,
        has_code: true,
    },
    // Rule 143
    RuleInfo {
        lhs: 23, /* expr */
        len: 5,
        has_code: true,
    },
    // Rule 144
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 145
    RuleInfo {
        lhs: 23, /* expr */
        len: 3,
        has_code: true,
    },
    // Rule 146
    RuleInfo {
        lhs: 23, /* expr */
        len: 1,
        has_code: false,
    },
    // Rule 147
    RuleInfo {
        lhs: 53, /* setlist */
        len: 2,
        has_code: true,
    },
    // Rule 148
    RuleInfo {
        lhs: 53, /* setlist */
        len: 2,
        has_code: false,
    },
    // Rule 149
    RuleInfo {
        lhs: 53, /* setlist */
        len: 1,
        has_code: false,
    },
    // Rule 150
    RuleInfo {
        lhs: 8, /* assign */
        len: 3,
        has_code: true,
    },
    // Rule 151
    RuleInfo {
        lhs: 8, /* assign */
        len: 3,
        has_code: true,
    },
    // Rule 152
    RuleInfo {
        lhs: 7, /* args */
        len: 2,
        has_code: true,
    },
    // Rule 153
    RuleInfo {
        lhs: 7, /* args */
        len: 2,
        has_code: true,
    },
    // Rule 154
    RuleInfo {
        lhs: 7, /* args */
        len: 4,
        has_code: true,
    },
    // Rule 155
    RuleInfo {
        lhs: 7, /* args */
        len: 2,
        has_code: true,
    },
    // Rule 156
    RuleInfo {
        lhs: 7, /* args */
        len: 0,
        has_code: true,
    },
    // Rule 157
    RuleInfo {
        lhs: 32, /* lnameargs */
        len: 3,
        has_code: true,
    },
    // Rule 158
    RuleInfo {
        lhs: 32, /* lnameargs */
        len: 1,
        has_code: false,
    },
    // Rule 159
    RuleInfo {
        lhs: 36, /* lvalue */
        len: 1,
        has_code: false,
    },
    // Rule 160
    RuleInfo {
        lhs: 36, /* lvalue */
        len: 3,
        has_code: true,
    },
    // Rule 161
    RuleInfo {
        lhs: 36, /* lvalue */
        len: 1,
        has_code: false,
    },
    // Rule 162
    RuleInfo {
        lhs: 43, /* nameargs */
        len: 3,
        has_code: true,
    },
    // Rule 163
    RuleInfo {
        lhs: 43, /* nameargs */
        len: 2,
        has_code: true,
    },
    // Rule 164
    RuleInfo {
        lhs: 43, /* nameargs */
        len: 4,
        has_code: true,
    },
    // Rule 165
    RuleInfo {
        lhs: 44, /* names */
        len: 3,
        has_code: true,
    },
    // Rule 166
    RuleInfo {
        lhs: 44, /* names */
        len: 1,
        has_code: true,
    },
    // Rule 167
    RuleInfo {
        lhs: 42, /* name */
        len: 3,
        has_code: true,
    },
    // Rule 168
    RuleInfo {
        lhs: 42, /* name */
        len: 1,
        has_code: true,
    },
    // Rule 169
    RuleInfo {
        lhs: 42, /* name */
        len: 1,
        has_code: false,
    },
    // Rule 170
    RuleInfo {
        lhs: 24, /* filename */
        len: 3,
        has_code: true,
    },
    // Rule 171
    RuleInfo {
        lhs: 24, /* filename */
        len: 1,
        has_code: false,
    },
    // Rule 172
    RuleInfo {
        lhs: 25, /* filepart */
        len: 1,
        has_code: false,
    },
    // Rule 173
    RuleInfo {
        lhs: 25, /* filepart */
        len: 1,
        has_code: false,
    },
    // Rule 174
    RuleInfo {
        lhs: 25, /* filepart */
        len: 1,
        has_code: false,
    },
    // Rule 175
    RuleInfo {
        lhs: 50, /* quoted */
        len: 2,
        has_code: true,
    },
    // Rule 176
    RuleInfo {
        lhs: 50, /* quoted */
        len: 0,
        has_code: true,
    },
    // Rule 177
    RuleInfo {
        lhs: 49, /* quotable */
        len: 1,
        has_code: true,
    },
    // Rule 178
    RuleInfo {
        lhs: 49, /* quotable */
        len: 1,
        has_code: true,
    },
    // Rule 179
    RuleInfo {
        lhs: 49, /* quotable */
        len: 1,
        has_code: true,
    },
];

pub static STATES: &[State] = &[
    // State 0
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 1),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (57, 54),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 1
    State {
        actions: None,
        default: Some(-1),
        gotos: &[],
    },
    // State 2
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13), (39, 75)]),
        default: None,
        gotos: &[(8, 18), (29, 74), (30, 29), (45, 57), (53, 76)],
    },
    // State 3
    State {
        actions: None,
        default: Some(-15),
        gotos: &[],
    },
    // State 4
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 85), (44, 78)],
    },
    // State 5
    State {
        actions: None,
        default: Some(-24),
        gotos: &[],
    },
    // State 6
    State {
        actions: Some(&[(8, -20)]),
        default: Some(-27),
        gotos: &[],
    },
    // State 7
    State {
        actions: None,
        default: Some(-6),
        gotos: &[],
    },
    // State 8
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 90),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 9
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 91),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 10
    State {
        actions: Some(&[(33, 92)]),
        default: None,
        gotos: &[],
    },
    // State 11
    State {
        actions: Some(&[(8, 93)]),
        default: None,
        gotos: &[],
    },
    // State 12
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 94),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 13
    State {
        actions: None,
        default: Some(-130),
        gotos: &[],
    },
    // State 14
    State {
        actions: None,
        default: Some(-39),
        gotos: &[],
    },
    // State 15
    State {
        actions: None,
        default: Some(-146),
        gotos: &[],
    },
    // State 16
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 95), (44, 78)],
    },
    // State 17
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13)]),
        default: None,
        gotos: &[(29, 96), (30, 29), (45, 57)],
    },
    // State 18
    State {
        actions: None,
        default: Some(-149),
        gotos: &[],
    },
    // State 19
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 97), (44, 78)],
    },
    // State 20
    State {
        actions: None,
        default: Some(-38),
        gotos: &[],
    },
    // State 21
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (45, 57), (55, 98)],
    },
    // State 22
    State {
        actions: None,
        default: Some(-36),
        gotos: &[],
    },
    // State 23
    State {
        actions: None,
        default: Some(-113),
        gotos: &[],
    },
    // State 24
    State {
        actions: Some(&[(11, 99), (24, 100)]),
        default: Some(-109),
        gotos: &[],
    },
    // State 25
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 101), (44, 78)],
    },
    // State 26
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 102),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 27
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 103),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 28
    State {
        actions: None,
        default: Some(-10),
        gotos: &[],
    },
    // State 29
    State {
        actions: Some(&[(3, 104)]),
        default: Some(-128),
        gotos: &[],
    },
    // State 30
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 107),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (29, 88),
            (30, 29),
            (34, 105),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 106),
        ],
    },
    // State 31
    State {
        actions: Some(&[(11, 108)]),
        default: Some(-112),
        gotos: &[],
    },
    // State 32
    State {
        actions: Some(&[(28, 81), (33, 109), (39, 110), (45, 83)]),
        default: Some(-87),
        gotos: &[
            (12, 111),
            (13, 112),
            (24, 114),
            (25, 77),
            (40, 113),
            (41, 115),
        ],
    },
    // State 33
    State {
        actions: None,
        default: Some(-21),
        gotos: &[],
    },
    // State 34
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 116),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 35
    State {
        actions: Some(&[
            (29, 121),
            (31, 118),
            (34, 122),
            (59, 119),
            (62, 120),
            (63, 117),
        ]),
        default: None,
        gotos: &[],
    },
    // State 36
    State {
        actions: None,
        default: Some(-104),
        gotos: &[],
    },
    // State 37
    State {
        actions: Some(&[(1, 48), (2, 21), (20, 124), (33, 13), (39, 75)]),
        default: Some(-19),
        gotos: &[(8, 123), (29, 74), (30, 29), (45, 57)],
    },
    // State 38
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 125), (44, 78)],
    },
    // State 39
    State {
        actions: None,
        default: Some(-37),
        gotos: &[],
    },
    // State 40
    State {
        actions: None,
        default: Some(-14),
        gotos: &[],
    },
    // State 41
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (66, 126),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (29, 88),
            (30, 29),
            (31, 129),
            (35, 42),
            (45, 57),
            (51, 130),
            (55, 128),
            (58, 127),
        ],
    },
    // State 42
    State {
        actions: None,
        default: Some(-103),
        gotos: &[],
    },
    // State 43
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 131), (44, 78)],
    },
    // State 44
    State {
        actions: None,
        default: Some(-78),
        gotos: &[(5, 132)],
    },
    // State 45
    State {
        actions: None,
        default: Some(-23),
        gotos: &[],
    },
    // State 46
    State {
        actions: Some(&[(8, 133)]),
        default: None,
        gotos: &[],
    },
    // State 47
    State {
        actions: None,
        default: Some(-176),
        gotos: &[(50, 134)],
    },
    // State 48
    State {
        actions: Some(&[(33, 135)]),
        default: None,
        gotos: &[],
    },
    // State 49
    State {
        actions: None,
        default: Some(-40),
        gotos: &[],
    },
    // State 50
    State {
        actions: Some(&[(33, 136)]),
        default: Some(-87),
        gotos: &[(12, 137), (40, 113), (41, 115)],
    },
    // State 51
    State {
        actions: None,
        default: Some(-8),
        gotos: &[],
    },
    // State 52
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13), (39, 75)]),
        default: None,
        gotos: &[(8, 18), (29, 74), (30, 29), (45, 57), (53, 138)],
    },
    // State 53
    State {
        actions: Some(&[(33, 136)]),
        default: None,
        gotos: &[(40, 113), (41, 139)],
    },
    // State 54
    State {
        actions: Some(&[(0, 140)]),
        default: None,
        gotos: &[],
    },
    // State 55
    State {
        actions: None,
        default: Some(-11),
        gotos: &[],
    },
    // State 56
    State {
        actions: None,
        default: Some(-42),
        gotos: &[],
    },
    // State 57
    State {
        actions: None,
        default: Some(-127),
        gotos: &[],
    },
    // State 58
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, -16),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-26),
        gotos: &[],
    },
    // State 59
    State {
        actions: Some(&[
            (65, 152),
            (1, 151),
            (2, 21),
            (28, 81),
            (33, 157),
            (39, 155),
            (45, 83),
        ]),
        default: None,
        gotos: &[
            (24, 87),
            (25, 77),
            (30, 156),
            (32, 153),
            (36, 154),
            (42, 86),
            (43, 158),
            (44, 78),
        ],
    },
    // State 60
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 159),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 61
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 31),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (8, 161),
            (23, 160),
            (29, 162),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 62
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13), (39, 163)]),
        default: Some(-119),
        gotos: &[(28, 166), (30, 165), (46, 164), (47, 167)],
    },
    // State 63
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-2),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (14, 55),
            (16, 168),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 64
    State {
        actions: None,
        default: Some(-22),
        gotos: &[],
    },
    // State 65
    State {
        actions: None,
        default: Some(-9),
        gotos: &[],
    },
    // State 66
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 169), (44, 78)],
    },
    // State 67
    State {
        actions: Some(&[(8, 170)]),
        default: None,
        gotos: &[],
    },
    // State 68
    State {
        actions: Some(&[
            (65, 152),
            (1, 151),
            (2, 21),
            (28, 81),
            (33, 157),
            (39, 155),
            (45, 83),
        ]),
        default: None,
        gotos: &[
            (24, 87),
            (25, 77),
            (30, 156),
            (32, 171),
            (36, 154),
            (42, 86),
            (43, 158),
            (44, 78),
        ],
    },
    // State 69
    State {
        actions: None,
        default: Some(-43),
        gotos: &[],
    },
    // State 70
    State {
        actions: None,
        default: Some(-12),
        gotos: &[],
    },
    // State 71
    State {
        actions: None,
        default: Some(-5),
        gotos: &[],
    },
    // State 72
    State {
        actions: None,
        default: Some(-13),
        gotos: &[],
    },
    // State 73
    State {
        actions: None,
        default: Some(-25),
        gotos: &[],
    },
    // State 74
    State {
        actions: Some(&[(11, 172), (24, 100)]),
        default: None,
        gotos: &[],
    },
    // State 75
    State {
        actions: Some(&[(11, 108)]),
        default: None,
        gotos: &[],
    },
    // State 76
    State {
        actions: Some(&[(1, 48), (2, 21), (20, 124), (33, 13), (39, 75)]),
        default: Some(-31),
        gotos: &[(8, 123), (29, 74), (30, 29), (45, 57)],
    },
    // State 77
    State {
        actions: None,
        default: Some(-171),
        gotos: &[],
    },
    // State 78
    State {
        actions: Some(&[(3, 173), (5, 175)]),
        default: Some(-156),
        gotos: &[(7, 174)],
    },
    // State 79
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13)]),
        default: None,
        gotos: &[(29, 176), (30, 29), (45, 57)],
    },
    // State 80
    State {
        actions: None,
        default: Some(-176),
        gotos: &[(50, 177)],
    },
    // State 81
    State {
        actions: None,
        default: Some(-172),
        gotos: &[],
    },
    // State 82
    State {
        actions: None,
        default: Some(-169),
        gotos: &[],
    },
    // State 83
    State {
        actions: None,
        default: Some(-174),
        gotos: &[],
    },
    // State 84
    State {
        actions: None,
        default: Some(-173),
        gotos: &[],
    },
    // State 85
    State {
        actions: None,
        default: Some(-34),
        gotos: &[],
    },
    // State 86
    State {
        actions: None,
        default: Some(-166),
        gotos: &[],
    },
    // State 87
    State {
        actions: Some(&[(24, 178)]),
        default: Some(-168),
        gotos: &[],
    },
    // State 88
    State {
        actions: Some(&[(24, 100)]),
        default: Some(-109),
        gotos: &[],
    },
    // State 89
    State {
        actions: None,
        default: Some(-112),
        gotos: &[],
    },
    // State 90
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-29),
        gotos: &[],
    },
    // State 91
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, 179),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 92
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 183),
            (13, 180),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (56, 38),
            (58, 67),
            (59, 9),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (8, 18),
            (9, 6),
            (10, 35),
            (18, 33),
            (21, 184),
            (23, 182),
            (26, 69),
            (29, 162),
            (30, 29),
            (33, 45),
            (35, 42),
            (39, 181),
            (45, 57),
            (48, 73),
            (53, 37),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (62, 56),
        ],
    },
    // State 93
    State {
        actions: None,
        default: Some(-7),
        gotos: &[],
    },
    // State 94
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
        ]),
        default: Some(-142),
        gotos: &[],
    },
    // State 95
    State {
        actions: None,
        default: Some(-41),
        gotos: &[],
    },
    // State 96
    State {
        actions: Some(&[(24, 100)]),
        default: Some(-110),
        gotos: &[],
    },
    // State 97
    State {
        actions: Some(&[(8, 185)]),
        default: None,
        gotos: &[],
    },
    // State 98
    State {
        actions: Some(&[(68, 186)]),
        default: None,
        gotos: &[],
    },
    // State 99
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (13, 180),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (56, 38),
            (58, 67),
            (59, 9),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (8, 18),
            (9, 6),
            (10, 35),
            (18, 33),
            (21, 184),
            (23, 187),
            (26, 69),
            (29, 162),
            (30, 29),
            (33, 45),
            (35, 42),
            (39, 188),
            (45, 57),
            (48, 73),
            (53, 37),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (62, 56),
        ],
    },
    // State 100
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 13), (45, 190)]),
        default: None,
        gotos: &[(30, 29), (45, 189)],
    },
    // State 101
    State {
        actions: Some(&[(8, 191)]),
        default: None,
        gotos: &[],
    },
    // State 102
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, 192),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 103
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-28),
        gotos: &[],
    },
    // State 104
    State {
        actions: None,
        default: Some(-156),
        gotos: &[(7, 193)],
    },
    // State 105
    State {
        actions: Some(&[(8, 194)]),
        default: None,
        gotos: &[],
    },
    // State 106
    State {
        actions: None,
        default: Some(-156),
        gotos: &[(7, 195)],
    },
    // State 107
    State {
        actions: Some(&[(11, 196), (35, 197)]),
        default: Some(-130),
        gotos: &[],
    },
    // State 108
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 198),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 109
    State {
        actions: Some(&[(11, 199)]),
        default: Some(-173),
        gotos: &[],
    },
    // State 110
    State {
        actions: None,
        default: Some(-85),
        gotos: &[],
    },
    // State 111
    State {
        actions: Some(&[(8, 200)]),
        default: None,
        gotos: &[],
    },
    // State 112
    State {
        actions: None,
        default: Some(-83),
        gotos: &[],
    },
    // State 113
    State {
        actions: None,
        default: Some(-99),
        gotos: &[],
    },
    // State 114
    State {
        actions: Some(&[(24, 178)]),
        default: Some(-84),
        gotos: &[],
    },
    // State 115
    State {
        actions: Some(&[(20, 201), (33, 136)]),
        default: Some(-86),
        gotos: &[(40, 202)],
    },
    // State 116
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, 203),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 117
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 204), (44, 78)],
    },
    // State 118
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 107),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (29, 88),
            (30, 29),
            (34, 205),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 106),
        ],
    },
    // State 119
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 206),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 120
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 207),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 121
    State {
        actions: Some(&[
            (65, 152),
            (1, 151),
            (2, 21),
            (28, 81),
            (33, 157),
            (39, 155),
            (45, 83),
        ]),
        default: None,
        gotos: &[
            (24, 87),
            (25, 77),
            (30, 156),
            (32, 208),
            (36, 154),
            (42, 86),
            (43, 158),
            (44, 78),
        ],
    },
    // State 122
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 209),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 123
    State {
        actions: None,
        default: Some(-147),
        gotos: &[],
    },
    // State 124
    State {
        actions: None,
        default: Some(-148),
        gotos: &[],
    },
    // State 125
    State {
        actions: None,
        default: Some(-35),
        gotos: &[],
    },
    // State 126
    State {
        actions: None,
        default: Some(-107),
        gotos: &[],
    },
    // State 127
    State {
        actions: None,
        default: Some(-116),
        gotos: &[],
    },
    // State 128
    State {
        actions: Some(&[(57, 210)]),
        default: Some(-104),
        gotos: &[],
    },
    // State 129
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (20, 211),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (66, 212),
            (67, 62),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (35, 42), (45, 57), (55, 36), (58, 213)],
    },
    // State 130
    State {
        actions: Some(&[(66, 214)]),
        default: None,
        gotos: &[],
    },
    // State 131
    State {
        actions: None,
        default: Some(-33),
        gotos: &[],
    },
    // State 132
    State {
        actions: Some(&[(8, 215)]),
        default: None,
        gotos: &[],
    },
    // State 133
    State {
        actions: None,
        default: Some(-76),
        gotos: &[(4, 216)],
    },
    // State 134
    State {
        actions: Some(&[(65, 221), (1, 48), (2, 21), (8, 220), (33, 13), (55, 219)]),
        default: None,
        gotos: &[(29, 217), (30, 29), (45, 57), (49, 218)],
    },
    // State 135
    State {
        actions: None,
        default: Some(-132),
        gotos: &[],
    },
    // State 136
    State {
        actions: Some(&[(11, 199)]),
        default: None,
        gotos: &[],
    },
    // State 137
    State {
        actions: Some(&[(8, 222)]),
        default: None,
        gotos: &[],
    },
    // State 138
    State {
        actions: Some(&[(1, 48), (2, 21), (20, 124), (33, 13), (39, 75)]),
        default: Some(-30),
        gotos: &[(8, 123), (29, 74), (30, 29), (45, 57)],
    },
    // State 139
    State {
        actions: Some(&[(20, 201), (33, 136)]),
        default: Some(-17),
        gotos: &[(40, 202)],
    },
    // State 140
    State {
        actions: None,
        default: Some(0),
        gotos: &[],
    },
    // State 141
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 223),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 142
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 224),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 143
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 225),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 144
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 226),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 145
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 227),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 146
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 228),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 147
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 229),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 148
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 230),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 149
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 231),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 150
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 232),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 151
    State {
        actions: Some(&[(1, 48), (2, 21), (33, 233)]),
        default: None,
        gotos: &[(29, 176), (30, 29), (45, 57)],
    },
    // State 152
    State {
        actions: None,
        default: Some(-176),
        gotos: &[(50, 234)],
    },
    // State 153
    State {
        actions: None,
        default: Some(-73),
        gotos: &[],
    },
    // State 154
    State {
        actions: Some(&[(11, 235)]),
        default: None,
        gotos: &[],
    },
    // State 155
    State {
        actions: Some(&[(11, -161)]),
        default: Some(-169),
        gotos: &[],
    },
    // State 156
    State {
        actions: None,
        default: Some(-159),
        gotos: &[],
    },
    // State 157
    State {
        actions: Some(&[(11, -130)]),
        default: Some(-173),
        gotos: &[],
    },
    // State 158
    State {
        actions: None,
        default: Some(-158),
        gotos: &[],
    },
    // State 159
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, 236),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 160
    State {
        actions: Some(&[
            (4, 237),
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 161
    State {
        actions: Some(&[(4, 238)]),
        default: None,
        gotos: &[],
    },
    // State 162
    State {
        actions: Some(&[(11, 172), (24, 100)]),
        default: Some(-109),
        gotos: &[],
    },
    // State 163
    State {
        actions: Some(&[(11, 239)]),
        default: None,
        gotos: &[],
    },
    // State 164
    State {
        actions: None,
        default: Some(-122),
        gotos: &[],
    },
    // State 165
    State {
        actions: Some(&[(11, 240)]),
        default: None,
        gotos: &[],
    },
    // State 166
    State {
        actions: Some(&[(68, 241)]),
        default: None,
        gotos: &[],
    },
    // State 167
    State {
        actions: Some(&[(1, 48), (2, 21), (20, 243), (33, 13), (39, 163)]),
        default: Some(-118),
        gotos: &[(30, 165), (46, 242)],
    },
    // State 168
    State {
        actions: None,
        default: Some(-4),
        gotos: &[],
    },
    // State 169
    State {
        actions: None,
        default: Some(-32),
        gotos: &[],
    },
    // State 170
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 244),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 171
    State {
        actions: Some(&[(8, 245)]),
        default: None,
        gotos: &[],
    },
    // State 172
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 246),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 173
    State {
        actions: None,
        default: Some(-156),
        gotos: &[(7, 247)],
    },
    // State 174
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-163),
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 175
    State {
        actions: Some(&[(65, 80), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 254)],
    },
    // State 176
    State {
        actions: Some(&[(24, 100)]),
        default: Some(-156),
        gotos: &[(7, 255)],
    },
    // State 177
    State {
        actions: Some(&[(65, 256), (1, 48), (2, 21), (8, 220), (33, 13), (55, 219)]),
        default: None,
        gotos: &[(29, 217), (30, 29), (45, 57), (49, 218)],
    },
    // State 178
    State {
        actions: Some(&[(28, 81), (33, 84), (45, 83)]),
        default: None,
        gotos: &[(25, 257)],
    },
    // State 179
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 258),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 180
    State {
        actions: Some(&[(8, 259)]),
        default: None,
        gotos: &[],
    },
    // State 181
    State {
        actions: None,
        default: Some(-91),
        gotos: &[],
    },
    // State 182
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-26),
        gotos: &[],
    },
    // State 183
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 260),
            (39, 31),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (8, 161),
            (23, 160),
            (29, 162),
            (30, 29),
            (35, 42),
            (38, 261),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 184
    State {
        actions: None,
        default: Some(-92),
        gotos: &[],
    },
    // State 185
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 262),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 186
    State {
        actions: None,
        default: Some(-131),
        gotos: &[],
    },
    // State 187
    State {
        actions: Some(&[
            (1, -150),
            (2, -150),
            (5, 150),
            (6, 144),
            (8, -150),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (20, -150),
            (23, 146),
            (33, -150),
            (39, -150),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-26),
        gotos: &[],
    },
    // State 188
    State {
        actions: None,
        default: Some(-89),
        gotos: &[],
    },
    // State 189
    State {
        actions: None,
        default: Some(-125),
        gotos: &[],
    },
    // State 190
    State {
        actions: None,
        default: Some(-126),
        gotos: &[],
    },
    // State 191
    State {
        actions: None,
        default: Some(-74),
        gotos: &[(3, 263)],
    },
    // State 192
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 264),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 193
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (4, 265),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 194
    State {
        actions: None,
        default: Some(-56),
        gotos: &[(1, 266)],
    },
    // State 195
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-64),
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 196
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (35, 42), (45, 57), (55, 36), (58, 267)],
    },
    // State 197
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (35, 42), (45, 57), (55, 36), (58, 268)],
    },
    // State 198
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-151),
        gotos: &[],
    },
    // State 199
    State {
        actions: Some(&[(65, 269), (39, 270), (45, 271)]),
        default: None,
        gotos: &[],
    },
    // State 200
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 272),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 201
    State {
        actions: None,
        default: Some(-98),
        gotos: &[],
    },
    // State 202
    State {
        actions: None,
        default: Some(-97),
        gotos: &[],
    },
    // State 203
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 273),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 204
    State {
        actions: None,
        default: Some(-66),
        gotos: &[],
    },
    // State 205
    State {
        actions: None,
        default: Some(-58),
        gotos: &[],
    },
    // State 206
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-47),
        gotos: &[],
    },
    // State 207
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-61),
        gotos: &[],
    },
    // State 208
    State {
        actions: None,
        default: Some(-81),
        gotos: &[],
    },
    // State 209
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-45),
        gotos: &[],
    },
    // State 210
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (45, 57), (55, 274)],
    },
    // State 211
    State {
        actions: None,
        default: Some(-115),
        gotos: &[],
    },
    // State 212
    State {
        actions: None,
        default: Some(-105),
        gotos: &[],
    },
    // State 213
    State {
        actions: None,
        default: Some(-114),
        gotos: &[],
    },
    // State 214
    State {
        actions: None,
        default: Some(-106),
        gotos: &[],
    },
    // State 215
    State {
        actions: Some(&[(55, 275)]),
        default: None,
        gotos: &[],
    },
    // State 216
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 276),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 217
    State {
        actions: Some(&[(24, 100)]),
        default: Some(-177),
        gotos: &[],
    },
    // State 218
    State {
        actions: None,
        default: Some(-175),
        gotos: &[],
    },
    // State 219
    State {
        actions: None,
        default: Some(-178),
        gotos: &[],
    },
    // State 220
    State {
        actions: None,
        default: Some(-179),
        gotos: &[],
    },
    // State 221
    State {
        actions: None,
        default: Some(-111),
        gotos: &[],
    },
    // State 222
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 1),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (57, 277),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 223
    State {
        actions: None,
        default: Some(-137),
        gotos: &[],
    },
    // State 224
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (7, 278),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 225
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
        ]),
        default: Some(-141),
        gotos: &[],
    },
    // State 226
    State {
        actions: Some(&[(23, 146), (42, 141)]),
        default: Some(-134),
        gotos: &[],
    },
    // State 227
    State {
        actions: Some(&[(5, 150), (6, 144), (23, 146), (42, 141)]),
        default: Some(-133),
        gotos: &[],
    },
    // State 228
    State {
        actions: Some(&[(42, 141)]),
        default: Some(-136),
        gotos: &[],
    },
    // State 229
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (12, 145),
            (19, 148),
            (23, 146),
            (42, 141),
        ]),
        default: Some(-139),
        gotos: &[],
    },
    // State 230
    State {
        actions: Some(&[(5, 150), (6, 144), (12, 145), (23, 146), (42, 141)]),
        default: Some(-138),
        gotos: &[],
    },
    // State 231
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
        ]),
        default: Some(-140),
        gotos: &[],
    },
    // State 232
    State {
        actions: Some(&[(6, 144), (23, 146), (42, 141)]),
        default: Some(-135),
        gotos: &[],
    },
    // State 233
    State {
        actions: Some(&[(11, -132)]),
        default: Some(-130),
        gotos: &[],
    },
    // State 234
    State {
        actions: Some(&[(65, 279), (1, 48), (2, 21), (8, 220), (33, 13), (55, 219)]),
        default: None,
        gotos: &[(29, 217), (30, 29), (45, 57), (49, 218)],
    },
    // State 235
    State {
        actions: Some(&[(65, 80), (1, 79), (28, 81), (33, 84), (39, 82), (45, 83)]),
        default: None,
        gotos: &[(24, 87), (25, 77), (42, 86), (43, 280), (44, 78)],
    },
    // State 236
    State {
        actions: None,
        default: Some(-59),
        gotos: &[(2, 281)],
    },
    // State 237
    State {
        actions: None,
        default: Some(-145),
        gotos: &[],
    },
    // State 238
    State {
        actions: None,
        default: Some(-144),
        gotos: &[],
    },
    // State 239
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 282),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 240
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 283),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 241
    State {
        actions: None,
        default: Some(-108),
        gotos: &[],
    },
    // State 242
    State {
        actions: None,
        default: Some(-120),
        gotos: &[],
    },
    // State 243
    State {
        actions: None,
        default: Some(-121),
        gotos: &[],
    },
    // State 244
    State {
        actions: Some(&[(17, 285), (30, 284)]),
        default: Some(-72),
        gotos: &[(27, 286)],
    },
    // State 245
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 287),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 246
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-150),
        gotos: &[],
    },
    // State 247
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (4, 288),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 248
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-152),
        gotos: &[],
    },
    // State 249
    State {
        actions: Some(&[(11, 239)]),
        default: Some(-112),
        gotos: &[],
    },
    // State 250
    State {
        actions: Some(&[(3, 104), (11, 240)]),
        default: Some(-128),
        gotos: &[],
    },
    // State 251
    State {
        actions: None,
        default: Some(-153),
        gotos: &[],
    },
    // State 252
    State {
        actions: Some(&[(11, 289), (24, 100)]),
        default: Some(-109),
        gotos: &[],
    },
    // State 253
    State {
        actions: None,
        default: Some(-155),
        gotos: &[],
    },
    // State 254
    State {
        actions: None,
        default: Some(-165),
        gotos: &[],
    },
    // State 255
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-162),
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 256
    State {
        actions: None,
        default: Some(-167),
        gotos: &[],
    },
    // State 257
    State {
        actions: None,
        default: Some(-170),
        gotos: &[],
    },
    // State 258
    State {
        actions: Some(&[(25, 292), (26, 290)]),
        default: Some(-50),
        gotos: &[(22, 291)],
    },
    // State 259
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 293),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 260
    State {
        actions: Some(&[(4, -96), (20, -96), (33, -96)]),
        default: Some(-130),
        gotos: &[],
    },
    // State 261
    State {
        actions: Some(&[(4, 296), (20, 294), (33, 295)]),
        default: None,
        gotos: &[],
    },
    // State 262
    State {
        actions: Some(&[(27, 297)]),
        default: None,
        gotos: &[],
    },
    // State 263
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 298),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 264
    State {
        actions: Some(&[(15, 299)]),
        default: Some(-55),
        gotos: &[(15, 300)],
    },
    // State 265
    State {
        actions: None,
        default: Some(-129),
        gotos: &[],
    },
    // State 266
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 301),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 267
    State {
        actions: None,
        default: Some(-156),
        gotos: &[(7, 302)],
    },
    // State 268
    State {
        actions: None,
        default: Some(-156),
        gotos: &[(7, 303)],
    },
    // State 269
    State {
        actions: Some(&[(55, 304)]),
        default: None,
        gotos: &[],
    },
    // State 270
    State {
        actions: None,
        default: Some(-100),
        gotos: &[],
    },
    // State 271
    State {
        actions: None,
        default: Some(-102),
        gotos: &[],
    },
    // State 272
    State {
        actions: Some(&[(27, 305)]),
        default: None,
        gotos: &[],
    },
    // State 273
    State {
        actions: Some(&[(25, 292), (26, 290)]),
        default: Some(-50),
        gotos: &[(22, 306)],
    },
    // State 274
    State {
        actions: None,
        default: Some(-117),
        gotos: &[],
    },
    // State 275
    State {
        actions: Some(&[(27, 307)]),
        default: None,
        gotos: &[],
    },
    // State 276
    State {
        actions: Some(&[(27, 308)]),
        default: None,
        gotos: &[],
    },
    // State 277
    State {
        actions: Some(&[(27, 309)]),
        default: None,
        gotos: &[],
    },
    // State 278
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 310),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 279
    State {
        actions: Some(&[(11, -160)]),
        default: Some(-167),
        gotos: &[],
    },
    // State 280
    State {
        actions: None,
        default: Some(-157),
        gotos: &[],
    },
    // State 281
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 311),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 282
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-123),
        gotos: &[],
    },
    // State 283
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-124),
        gotos: &[],
    },
    // State 284
    State {
        actions: Some(&[(8, 312)]),
        default: None,
        gotos: &[],
    },
    // State 285
    State {
        actions: Some(&[(8, 315), (22, 313), (28, 81), (33, 84), (45, 83)]),
        default: None,
        gotos: &[(24, 314), (25, 77)],
    },
    // State 286
    State {
        actions: Some(&[(27, 316)]),
        default: None,
        gotos: &[],
    },
    // State 287
    State {
        actions: Some(&[(27, 317)]),
        default: None,
        gotos: &[],
    },
    // State 288
    State {
        actions: None,
        default: Some(-164),
        gotos: &[],
    },
    // State 289
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 318),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 290
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (33, 13),
            (39, 89),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (23, 319),
            (29, 88),
            (30, 29),
            (35, 42),
            (45, 57),
            (55, 36),
            (58, 15),
        ],
    },
    // State 291
    State {
        actions: Some(&[(27, 320)]),
        default: None,
        gotos: &[],
    },
    // State 292
    State {
        actions: Some(&[(8, 321)]),
        default: None,
        gotos: &[],
    },
    // State 293
    State {
        actions: Some(&[(27, 322)]),
        default: None,
        gotos: &[],
    },
    // State 294
    State {
        actions: None,
        default: Some(-95),
        gotos: &[],
    },
    // State 295
    State {
        actions: None,
        default: Some(-94),
        gotos: &[],
    },
    // State 296
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (13, 180),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (56, 38),
            (58, 67),
            (59, 9),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[
            (8, 18),
            (9, 6),
            (10, 35),
            (18, 33),
            (21, 184),
            (23, 182),
            (26, 69),
            (29, 162),
            (30, 29),
            (33, 45),
            (35, 42),
            (39, 323),
            (45, 57),
            (48, 73),
            (53, 37),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (62, 56),
        ],
    },
    // State 297
    State {
        actions: None,
        default: Some(-65),
        gotos: &[],
    },
    // State 298
    State {
        actions: Some(&[(27, 324)]),
        default: None,
        gotos: &[],
    },
    // State 299
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (8, 326),
            (22, 327),
            (33, 13),
            (39, 89),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: None,
        gotos: &[(29, 88), (30, 29), (35, 42), (45, 57), (55, 36), (58, 325)],
    },
    // State 300
    State {
        actions: Some(&[(27, 328)]),
        default: None,
        gotos: &[],
    },
    // State 301
    State {
        actions: Some(&[(27, 329)]),
        default: None,
        gotos: &[],
    },
    // State 302
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-62),
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 303
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (20, 253),
            (33, 13),
            (39, 249),
            (44, 12),
            (45, 23),
            (50, 17),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-63),
        gotos: &[
            (23, 248),
            (29, 252),
            (30, 250),
            (35, 42),
            (45, 57),
            (46, 251),
            (55, 36),
            (58, 15),
        ],
    },
    // State 304
    State {
        actions: Some(&[(65, 330)]),
        default: None,
        gotos: &[],
    },
    // State 305
    State {
        actions: None,
        default: Some(-88),
        gotos: &[],
    },
    // State 306
    State {
        actions: Some(&[(27, 331)]),
        default: None,
        gotos: &[],
    },
    // State 307
    State {
        actions: None,
        default: Some(-79),
        gotos: &[],
    },
    // State 308
    State {
        actions: None,
        default: Some(-77),
        gotos: &[],
    },
    // State 309
    State {
        actions: None,
        default: Some(-82),
        gotos: &[],
    },
    // State 310
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-143),
        gotos: &[],
    },
    // State 311
    State {
        actions: Some(&[(27, 332)]),
        default: None,
        gotos: &[],
    },
    // State 312
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 333),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 313
    State {
        actions: Some(&[(8, 334)]),
        default: None,
        gotos: &[],
    },
    // State 314
    State {
        actions: Some(&[(8, 335), (24, 178)]),
        default: None,
        gotos: &[],
    },
    // State 315
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 336),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 316
    State {
        actions: None,
        default: Some(-67),
        gotos: &[],
    },
    // State 317
    State {
        actions: None,
        default: Some(-80),
        gotos: &[],
    },
    // State 318
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: Some(-154),
        gotos: &[],
    },
    // State 319
    State {
        actions: Some(&[
            (5, 150),
            (6, 144),
            (8, 337),
            (9, 142),
            (10, 149),
            (12, 145),
            (16, 147),
            (19, 148),
            (23, 146),
            (42, 141),
            (46, 143),
        ]),
        default: None,
        gotos: &[],
    },
    // State 320
    State {
        actions: None,
        default: Some(-46),
        gotos: &[],
    },
    // State 321
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 338),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 322
    State {
        actions: None,
        default: Some(-93),
        gotos: &[],
    },
    // State 323
    State {
        actions: None,
        default: Some(-90),
        gotos: &[],
    },
    // State 324
    State {
        actions: None,
        default: Some(-75),
        gotos: &[],
    },
    // State 325
    State {
        actions: Some(&[(8, 339)]),
        default: None,
        gotos: &[],
    },
    // State 326
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 340),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 327
    State {
        actions: Some(&[(8, 341)]),
        default: None,
        gotos: &[],
    },
    // State 328
    State {
        actions: None,
        default: Some(-51),
        gotos: &[],
    },
    // State 329
    State {
        actions: None,
        default: Some(-57),
        gotos: &[],
    },
    // State 330
    State {
        actions: None,
        default: Some(-101),
        gotos: &[],
    },
    // State 331
    State {
        actions: None,
        default: Some(-44),
        gotos: &[],
    },
    // State 332
    State {
        actions: None,
        default: Some(-60),
        gotos: &[],
    },
    // State 333
    State {
        actions: None,
        default: Some(-71),
        gotos: &[],
    },
    // State 334
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 342),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 335
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 343),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 336
    State {
        actions: Some(&[(17, 285), (30, 284)]),
        default: Some(-72),
        gotos: &[(27, 344)],
    },
    // State 337
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 345),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 338
    State {
        actions: None,
        default: Some(-49),
        gotos: &[],
    },
    // State 339
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 346),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 340
    State {
        actions: None,
        default: Some(-54),
        gotos: &[],
    },
    // State 341
    State {
        actions: Some(&[
            (65, 47),
            (1, 48),
            (2, 21),
            (3, 61),
            (8, -18),
            (13, 32),
            (14, 8),
            (18, 20),
            (21, 16),
            (22, 2),
            (29, 68),
            (31, 30),
            (32, 27),
            (33, 13),
            (34, 34),
            (36, 43),
            (37, 66),
            (38, 14),
            (39, 31),
            (40, 10),
            (41, 53),
            (43, 49),
            (44, 12),
            (45, 23),
            (47, 46),
            (48, 4),
            (49, 44),
            (50, 17),
            (51, 22),
            (52, 52),
            (53, 39),
            (54, 26),
            (55, 7),
            (56, 38),
            (58, 67),
            (59, 9),
            (60, 59),
            (61, 25),
            (62, 60),
            (63, 19),
            (64, 41),
            (67, 62),
        ]),
        default: Some(-3),
        gotos: &[
            (6, 28),
            (8, 18),
            (9, 6),
            (10, 35),
            (11, 347),
            (14, 55),
            (16, 71),
            (17, 63),
            (18, 33),
            (19, 65),
            (20, 50),
            (21, 51),
            (23, 58),
            (26, 69),
            (29, 24),
            (30, 29),
            (33, 45),
            (35, 42),
            (37, 70),
            (45, 57),
            (48, 73),
            (52, 3),
            (53, 37),
            (54, 11),
            (55, 36),
            (56, 64),
            (58, 15),
            (59, 5),
            (60, 72),
            (61, 40),
            (62, 56),
        ],
    },
    // State 342
    State {
        actions: Some(&[(17, 285), (30, 284)]),
        default: Some(-72),
        gotos: &[(27, 348)],
    },
    // State 343
    State {
        actions: Some(&[(17, 285), (30, 284)]),
        default: Some(-72),
        gotos: &[(27, 349)],
    },
    // State 344
    State {
        actions: None,
        default: Some(-70),
        gotos: &[],
    },
    // State 345
    State {
        actions: Some(&[(25, 292), (26, 290)]),
        default: Some(-50),
        gotos: &[(22, 350)],
    },
    // State 346
    State {
        actions: Some(&[(15, 299)]),
        default: Some(-55),
        gotos: &[(15, 351)],
    },
    // State 347
    State {
        actions: None,
        default: Some(-53),
        gotos: &[],
    },
    // State 348
    State {
        actions: None,
        default: Some(-69),
        gotos: &[],
    },
    // State 349
    State {
        actions: None,
        default: Some(-68),
        gotos: &[],
    },
    // State 350
    State {
        actions: None,
        default: Some(-48),
        gotos: &[],
    },
    // State 351
    State {
        actions: None,
        default: Some(-52),
        gotos: &[],
    },
];
