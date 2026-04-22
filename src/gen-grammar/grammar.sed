s/^ *| /    /g
s/seplist/list/g
s/<typ_id>/<id>/g
/^<id> ::=/,+2d
s/<id>/ID/g
/^<semicolon> ::=/,+3d
/^<annot(T)> ::=/,+2d
/^<pat_opt> ::=/,+3d
/^<start> ::=/,+2d
/^<parse_prog_interactive> ::=/,+3d
/^<import_list> ::=/,+2d
/^<parse_module_header> ::=/,+2d
/^<typ_dec> ::=/,/^$/d
/^<stab_field> ::=/,/^$/d
/^<pre_stab_field> ::=/,/^$/d
/^<mig_lab> ::=/,/^$/d
/^<mig_field> ::=/,/^$/d
/^<req> ::=/,/^$/d
/^<parse_stab_sig> ::=/,/^$/d
/.*PRIM.*/d
/.*NUM_DOT_ID.*/d
/^<bl> ::=/,+2d
/^<ob> ::=/,+2d
s/<start> //g
s/<parse_prog>/<prog>/g
s/(<bl>)//g
s/(<ob>)//g
s/(B)//g
s/ B$/ <exp_obj>/g
s/\[/(/g
s/\]/)?/g
s/(\([a-zA-Z_0-9]*\))/\1/g
s/(\(<[a-z_0-9]*>\))/\1/g
s/<semicolon>/\';\'/g
s/<annot_opt>/(':' <typ>)?/g
s/<pat_opt>/<pat_plain>?/g
s/epsilon/<empty>/g
s/WRAPADDASSIGN/\'+%=\'/g
s/WRAPSUBASSIGN/\'-%=\'/g
s/WRAPMULASSIGN/\'*%=\'/g
s/WRAPPOWASSIGN/\'**%=\'/g
s/WRAPADDOP/\'+%\'/g
s/WRAPSUBOP/\'-%\'/g
s/WRAPMULOP/\'*%\'/g
s/WRAPPOWOP/\'**%\'/g
s/ANDASSIGN/\'\&=\'/g
s/ACTOR/\'actor\'/g
s/COMPOSITE/\'composite\'/g
s/IGNORE/\'ignore\'/g
s/IMPORT/\'import\'/g
s/IMPLICIT/\'implicit\'/g
s/XOROP/\'^\'/g
s/XORASSIGN/\'^=\'/g
s/WHILE/\'while\'/g
s/VAR/\'var\'/g
s/SHROP/\' >>\'/g
s/SHRASSIGN/\'>>=\'/g
s/UNDERSCORE/\'_\'/g
s/TYPE/\'type\'/g
s/TRANSIENT/\'transient\'/g
s/TRY/\'try\'/g
s/THROW/\'throw\'/g
s/FINALLY/\'finally\'/g
s/TEXT/<text>/g
s/SWITCH/\'switch\'/g
s/SUBOP/\'-\'/g
s/SUB/\'<:\'/g
s/STABLE/\'stable\'/g
s/SHLOP/\'<<\'/g
s/SHLASSIGN/\'<<=\'/g
s/SHARED/\'shared\'/g
s/SYSTEM/\'system\'/g
s/RPAR/\')\'/g
s/ROTROP/\'<>>\'/g
s/ROTRASSIGN/\'<>>=\'/g
s/ROTLOP/\'<<>\'/g
s/ROTLASSIGN/\'<<>=\'/g
s/RETURN/\'return\'/g
s/RCURLY/\'}\'/g
s/RBRACKET/\']\'/g
s/AWAITQUEST/\'await?\'/g
s/QUEST/\'?\'/g
s/BANG/\'!\'/g
s/QUERY/\'query\'/g
s/PERSISTENT/\'persistent\'/g
s/PIPE/\'|>\'/g
s/PUBLIC/\'public\'/g
s/PRIVATE/\'private\'/g
s/POWOP/\'**\'/g
s/POWASSIGN/\'**-\'/g
s/PLUSASSIGN/\'+=\'/g
s/OROP/\'|\'/g
s/ORASSIGN/\'|=\'/g
s/OBJECT/\'object\'/g
s/NULLCOALESCE/'\?\?'/g
s/NULL/\'null\'/g
s/NOT/\'not\'/g
s/NEQOP/\'!=\'/g
s/NAT/<nat>/g
s/MULOP/\'*\'/g
s/MULASSIGN/\'*=\'/g
s/MODULE/\'module\'/g
s/MODOP/\'%\'/g
s/MODASSIGN/\'%=\'/g
s/MINUSASSIGN/\'-=\'/g
s/LTOP/\' < \'/g
s/LT/\'<\'/g
s/LPAR/\'(\'/g
s/LOOP/\'loop\'/g
s/LET/\'let\'/g
s/LEOP/\'<=\'/g
s/LCURLY/\'{\'/g
s/LBRACKET/\'[\'/g
s/LABEL/\'label\'/g
s/CONTINUE/\'continue\'/g
s/MIXIN/\'mixin\'/g
s/INCLUDE/\'include\'/g
s/IN/\'in\'/g
s/IF/\'if\'/g
s/TO_CANDID/\'to_candid\'/g
s/FROM_CANDID/\'from_candid\'/g
s/ID/<id>/g
s/HASH/\'#\'/g
s/GTOP/\' > \'/g
s/GT/\'>\'/g
s/GEOP/\'>=\'/g
s/FUNC/\'func\'/g
s/FOR/\'for\'/g
s/FLEXIBLE/\'flexible\'/g
s/FLOAT/<float>/g
s/EQOP/\'==\'/g
s/EQ/\'=\'/g
s/EOF//g
s/ELSE/\'else\'/g
s/DOT_NUM/\'.\'<nat>/g
s/DOT/\'.\'/g
s/DIVOP/\'\/\'/g
s/DIVASSIGN/\'\/=\'/g
s/DEBUG_SHOW/\'debug_show\'/g
s/DEBUG/\'debug\'/g
s/COMMA/\',\'/g
s/COLON/\':\'/g
s/CLASS/\'class\'/g
s/CHAR/<char>/g
s/CATCH/\'catch\'/g
s/CATASSIGN/\'@=\'/g
s/CASE/\'case\'/g
s/BREAK/\'break\'/g
s/BOOL/<bool>/g
s/AWAITSTAR/\'await*\'/g
s/AWAIT/\'await\'/g
s/ASYNCSTAR/\'async*\'/g
s/ASYNC/\'async\'/g
s/ASSERT/\'assert\'/g
s/ARROW/\'->\'/g
s/ANDOP/\'\&\'/g
s/ADDOP/\'+\'/g
s/ASSIGN/\':=\'/g
s/DO/\'do\'/g
s/OR/\'or\'/g
s/AND/\'and\'/g
s/WITH/\'with\'/g
s/WEAK/\'weak\'/g
/'return'$/d
s/'return' <exp>/'return' <exp>?/
