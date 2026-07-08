WS        : [ \t\r\n]+ -> skip ;
LINE_CMT  : '//' ~[\n]* -> comment ;
BLOCK_CMT : '/*' ( !'*/' . )* '*/' -> comment ;
STRING    : '"' ( '\\' . / ~["\\] )* '"' ;
CHAR      : '\'' ( '\\' . / ~['\\] ) '\'' ;
IDENT     : [a-zA-Z_] [a-zA-Z0-9_]* ;
LBRACE    : '{' ;
RBRACE    : '}' ;
LPAREN    : '(' ;
RPAREN    : ')' ;
LBRACK    : '[' ;
RBRACK    : ']' ;
OTHER     : . ;

unit : item ;

item : outer* vis core ;
outer : '#' '!'? LBRACK loose* RBRACK ;
vis : ( 'pub' pgroup? )? ;
core : modItem / nestItem / fnItem / structItem / lineItem ;

modItem    : quals 'impl' head LBRACE unit* RBRACE -> item ;
nestItem   : quals ( 'trait' / 'mod' ) name head LBRACE unit* RBRACE -> item ;
fnItem     : quals 'fn' name head ( scope / ';' ) -> item ;
structItem : quals ( 'struct' / 'enum' / 'union' ) name head structTail -> item ;
lineItem   : lineKw ( !';' loose )* ';'? -> item ;
name       : IDENT -> word ;
quals      : ( 'async' / 'unsafe' / 'const' / 'extern' / 'default' / 'move' )* ;
lineKw     : 'use' / 'static' / 'type' / 'const' / 'extern' / 'mod' ;

structTail : fields / ';' ;
fields : LBRACE member* RBRACE ;
member : outer* vis? name membertail ;
membertail : ( !',' !RBRACE loose )* ','? ;

stmt : control / letStmt / structLit / scope / ( !RBRACE . ) ;
letStmt : 'let' 'mut'? name !!( '=' / ':' / ';' ) ;
control : ifExpr / whileExpr / forExpr / loopExpr / matchExpr ;
ifExpr    : 'if' head scope elseTail ;
elseTail  : ( 'else' ( ifExpr / scope ) )? ;
whileExpr : 'while' head scope ;
forExpr   : 'for' head scope ;
loopExpr  : 'loop' scope ;
matchExpr : 'match' head scope ;

scope : LBRACE stmt* RBRACE -> scope ;
structLit : path LBRACE loose* RBRACE -> literal ;
path : IDENT ( ':' ':' IDENT )* ;

head : headAtom* ;
headAtom : pgroup / bgroup / ( !LBRACE !RBRACE !';' . ) ;
pgroup : LPAREN inner* RPAREN ;
bgroup : LBRACK inner* RBRACK ;
inner  : pgroup / bgroup / scope / structLit / ( !RPAREN !RBRACK . ) ;

loose : group / ( !LBRACE !RBRACE !LBRACK !RBRACK !LPAREN !RPAREN . ) ;
group : LBRACE loose* RBRACE / LBRACK loose* RBRACK / LPAREN loose* RPAREN ;
