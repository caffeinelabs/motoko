open Ir_def
open Mo_def
open Mo_types
open Mo_values
open Syntax

open Source
open Operator
module S = Syntax
module I = Ir
module T = Type
open Construct

(*
As a first scaffolding, we translate imported files into let-bound
variables with a special, non-colliding name, which we sometimes
want to recognize for better user experience.
*)

let id_of_full_path (fp : string) : string =
  "file$" ^ fp

(* Combinators used in the desugaring *)

let apply_sign op l = Syntax.(match op, l with
  | PosOp, l -> l
  | NegOp, (NatLit n | IntLit n) -> IntLit Numerics.Int.(sub zero n)
  | NegOp, Int8Lit n -> Int8Lit Numerics.Int_8.(sub zero n)
  | NegOp, Int16Lit n -> Int16Lit Numerics.Int_16.(sub zero n)
  | NegOp, Int32Lit n -> Int32Lit Numerics.Int_32.(sub zero n)
  | NegOp, Int64Lit n -> Int64Lit Numerics.Int_64.(sub zero n)
  | _, _ -> raise (Invalid_argument "Invalid signed pattern")
  )

let phrase f x = { x with it = f x.it }

let typ_note : S.typ_note -> Note.t =
  fun S.{ note_typ; note_eff; _ } -> Note.{ def with typ = note_typ; eff = note_eff }

let phrase' f x =
  { x with it = f x.at x.note x.it }

let typed_phrase' f x =
  let n' = typ_note x.note in
  { x with it = f x.at n' x.it; note = n' }

let is_empty_tup e = e.it = S.TupE []

let unit_typ at = { it = S.TupT []; at; note = T.unit }

let desugar_loop_flags at note body flags with_body =
  let { has_break; has_continue } = flags in
  let body = if not has_continue then body else
    let () = flags.has_continue <- false in
    { body with it = S.LabelE (S.auto_continue_s @@ body.at, unit_typ body.at, body) }
  in
  if not has_break then `Body body else
  let () = flags.has_break <- false in
  `Rec (S.LabelE (S.auto_s @@ at, unit_typ at, { it = with_body body; at; note = { note_typ = note.Note.typ; note_eff = note.Note.eff } }))

let rec exps es = List.map exp es

and exp e =
    (* We short-cut AnnotE here, so that we get the position of the inner expression *)
    match e.it with
    | S.AnnotE (e', t) -> exp e'
    | _ -> typed_phrase' exp' e

and exp' at note = function
  | S.HoleE (_, e) -> (exp !e).it
  | S.VarE i ->
    (match i.note with
     | (mut, None) ->
       I.VarE ((match mut with Var -> I.Var | Const -> I.Const), i.it)
     | (_, Some e) -> (* auto-import *)
       (exp e).it)
  | S.ActorUrlE e ->
    I.(PrimE (ActorOfIdBlob note.Note.typ, [url e at]))
  | S.LitE l -> I.LitE (lit !l)
  | S.UnE (ot, o, e) ->
    I.PrimE (I.UnPrim (!ot, o), [exp e])
  | S.BinE (ot, e1, o, e2) ->
    I.PrimE (I.BinPrim (!ot, o), [exp e1; exp e2])
  | S.RelE (ot, e1, Operator.NeqOp, e2) ->
    (notE (primE (I.RelPrim (!ot, Operator.EqOp)) [exp e1; exp e2])).it
  | S.RelE (ot, e1, o, e2) ->
    I.PrimE (I.RelPrim (!ot, o), [exp e1; exp e2])
  | S.ShowE (ot, e) ->
    I.PrimE (I.ShowPrim !ot, [exp e])
  | S.ToCandidE es ->
    let args = exps es in
    let ts = List.map (fun e -> e.note.Note.typ) args in
    (primE (I.SerializePrim ts) [seqE args]).it
  | S.FromCandidE e ->
    begin match T.normalize note.Note.typ with
    | T.Opt t -> (primE (I.DeserializeOptPrim (T.as_seq t)) [exp e]).it
    | _ -> assert false
    end
  | S.TupE es -> (tupE (exps es)).it
  | S.ProjE (e, i) -> (projE (exp e) i).it
  | S.OptE e -> (optE (exp e)).it
  | S.DoOptE e ->
    I.LabelE ("!", note.Note.typ, optE (exp e))
  | S.BangE e ->
    let ty = note.Note.typ in
    let v = fresh_var "v" ty in
    (switch_optE (exp e)
      (* case null : *)
      (breakE "!" (nullE()))
      (* case ? v : *)
      (varP v) (varE v) ty).it
  | S.ObjBlockE (exp_opt, s, (self_id_opt, _), dfs) ->
    let eo = Option.map exp exp_opt in
    obj_block at s eo self_id_opt dfs note.Note.typ
  | S.ObjE (bs, efs) ->
    obj note.Note.typ efs bs
  | S.TagE (c, e) -> (tagE c.it (exp e)).it
  | S.DotE (e, x, _) when T.is_array e.note.S.note_typ ->
    (array_dotE e.note.S.note_typ x.it (exp e)).it
  | S.DotE (e, x, _) when T.is_prim T.Blob e.note.S.note_typ ->
    (blob_dotE x.it (exp e)).it
  | S.DotE (e, x, _) when T.is_prim T.Text e.note.S.note_typ ->
    (text_dotE x.it (exp e)).it
  | S.DotE (e, x, _) ->
    begin match T.as_obj_sub [x.it] e.note.S.note_typ with
    | T.Actor, _ -> I.PrimE (I.ActorDotPrim x.it, [exp e])
    | _ -> I.PrimE (I.DotPrim x.it, [exp e])
    end
  | S.AssignE (e1, e2) -> I.AssignE (lexp e1, exp e2)
  | S.ArrayE (m, es) ->
    let t = T.as_array note.Note.typ in
    I.PrimE (I.ArrayPrim (mut m, T.as_immut t), exps es)
  | S.IdxE (e1, e2) when e1.note.S.note_typ = T.blob -> I.PrimE (I.IdxBlobPrim, [exp e1; exp e2])
  | S.IdxE (e1, e2) -> I.PrimE (I.IdxPrim, [exp e1; exp e2])
  | S.FuncE (name, sp, tbs, p, _t_opt, _, e) ->
    let s, po = match sp.it with
      | T.Local -> (T.Local, None)
      | T.Shared (ss, {it = S.WildP; _} ) -> (* don't bother with ctxt pat *)
        (T.Shared ss, None)
      | T.Shared (ss, sp) -> (T.Shared ss, Some sp) in
    let args, _, wrap, control, res_tys = to_args note.Note.typ po None p in
    let tbs' = typ_binds tbs in
    let vars = List.map (fun (tb : I.typ_bind) -> T.Con (tb.it.I.con, [])) tbs' in
    let tys = List.map (T.open_ vars) res_tys in
    I.FuncE (name, s, control, tbs', args, tys, wrap (exp e))
  (* Primitive functions in the prelude have particular shapes *)
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE p;_}, _);note;_}, _, (_, e))
    when Lib.String.chop_prefix "num_conv" p <> None ->
    begin match String.split_on_char '_' p with
    | ["num"; "conv"; s1; s2] ->
      let p1 = Type.prim s1 in
      let p2 = Type.prim s2 in
      I.PrimE (I.NumConvTrapPrim (p1, p2), [exp !e])
    | _ -> assert false
    end
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE p;_}, _);note;_}, _, (_, e))
    when Lib.String.chop_prefix "num_wrap" p <> None ->
    begin match String.split_on_char '_' p with
    | ["num"; "wrap"; s1; s2] ->
      let p1 = Type.prim s1 in
      let p2 = Type.prim s2 in
      I.PrimE (I.NumConvWrapPrim (p1, p2), [exp !e])
    | _ -> assert false
    end
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "decodeUtf8";_},_);_}, _, (_, e)) ->
    I.PrimE (I.DecodeUtf8, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "encodeUtf8";_},_);_}, _, (_, e)) ->
    I.PrimE (I.EncodeUtf8, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cast";_}, _);note;_}, _, (_, e)) ->
    begin match note.S.note_typ with
    | T.Func (T.Local, T.Returns, [], ts1, ts2) ->
      I.PrimE (I.CastPrim (T.seq ts1, T.seq ts2), [exp !e])
    | _ -> assert false
    end
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "serialize";_}, _);note;_}, _, (_, e)) ->
    begin match note.S.note_typ with
    | T.Func (T.Local, T.Returns, [], ts1, ts2) ->
      I.PrimE (I.SerializePrim ts1, [exp !e])
    | _ -> assert false
    end
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "deserialize";_}, _);note;_}, _, (_, e)) ->
    begin match note.S.note_typ with
    | T.Func (T.Local, T.Returns, [], ts1, ts2) ->
      I.PrimE (I.DeserializePrim ts2, [exp (!e)])
    | _ -> assert false
    end
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "caller";_},_);_}, _, (_, e)) ->
    (match !e with
     | {it=S.TupE [];_} ->
        I.PrimE (I.ICCallerPrim, [])
     | _ -> assert false)
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "deadline";_},_);_}, _, (_, e)) ->
    assert ((!e).it = S.TupE []);
    I.PrimE (I.ICReplyDeadlinePrim, [])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "time";_},_);_}, _, (_, e)) ->
    assert ((!e).it = S.TupE []);
    I.PrimE (I.SystemTimePrim, [])
  (* Cycles *)
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesBalance";_},_);_}, _, (_, e)) ->
    assert (is_empty_tup !e);
    I.PrimE (I.SystemCyclesBalancePrim, [])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesAvailable";_},_);_}, _, (_, e)) ->
    assert (is_empty_tup !e);
    I.PrimE (I.SystemCyclesAvailablePrim, [])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesRefunded";_},_);_}, _, (_, e)) ->
    assert (is_empty_tup !e);
    I.PrimE (I.SystemCyclesRefundedPrim, [])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesAccept";_},_);_}, _, (_, e)) ->
    I.PrimE (I.SystemCyclesAcceptPrim, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesAdd";_},_);_}, _, (_, e)) ->
    I.PrimE (I.SystemCyclesAddPrim, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "cyclesBurn";_},_);_}, _, (_, e)) ->
    I.PrimE (I.SystemCyclesBurnPrim, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "timeoutSet";_},_);_}, _, (_, e)) ->
    I.PrimE (I.SystemTimeoutSetPrim, [exp !e])
  (* Certified data *)
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "setCertifiedData";_},_);_}, _, (_, e)) ->
    I.PrimE (I.SetCertifiedData, [exp !e])
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE "getCertificate";_},_);_}, _, (_, e)) ->
    assert (is_empty_tup !e);
    I.PrimE (I.GetCertificate, [])
  (* Other *)
  | S.CallE (None, {it=S.AnnotE ({it=S.PrimE p;_},_);_}, _, (_, e)) ->
   (match (!e).it with
    | S.TupE es ->
      I.PrimE (I.OtherPrim p, exps es)
    | _ ->
      I.PrimE (I.OtherPrim p, [exp !e]))
  (* Optimizing array.size() *)
  | S.CallE (None, {it=S.DotE (e1, proj, _); _}, _, (_, e))
     when is_empty_tup !e &&
          T.is_array e1.note.S.note_typ && proj.it = "size" ->
    I.PrimE (I.OtherPrim "array_len", [exp e1])
  | S.CallE (None, {it=S.DotE (e1, proj, _); _}, _, (_, e))
      when is_empty_tup !e &&
           T.(is_prim Text) e1.note.S.note_typ && proj.it = "size" ->
    I.PrimE (I.OtherPrim "text_len", [exp e1])
  | S.CallE (None, {it=S.DotE (e1, proj,_); _}, _, (_, e))
      when is_empty_tup !e &&
           T.(is_prim Blob) e1.note.S.note_typ && proj.it = "size" ->
    I.PrimE (I.OtherPrim "blob_size", [exp e1])
  (* Contextual dot call *)
  | S.CallE (None, {it=S.DotE(e1, id, n);_}, inst, (_, e2)) when Option.is_some !n ->
     let func_exp = Option.get !n in
     let args = S.contextual_dot_args e1 !e2 func_exp in
     I.(PrimE (CallPrim inst.note, [exp func_exp; exp args]))
  (* Normal call *)
  | S.CallE (None, e1, inst, (_, e2)) ->
    I.(PrimE (CallPrim inst.note, [exp e1; exp !e2]))
  (* Call with parenthetical *)
  | S.CallE (Some _ as par_opt, e1, inst, (_, e2)) ->
    let send e1_typ = T.(is_func e1_typ &&
                           (let s, _, _, _, _ = as_func e1_typ in
                            is_shared_sort s || is_fut note.Note.typ)) in
    let ds, rs = parenthetical (send e1.note.S.note_typ) par_opt in
    let v1, v2 = fresh_var "e1" e1.note.S.note_typ, fresh_var "e2" (!e2).note.S.note_typ in
    (blockE
       (ds @ letD v1 (exp e1) :: letD v2 (exp !e2) :: rs)
       I.{ at; note; it = PrimE (CallPrim inst.note, [varE v1; varE v2]) }).it
  | S.BlockE [] -> (unitE ()).it
  | S.BlockE [{it = S.ExpD e; _}] -> (exp e).it
  | S.BlockE ds -> I.BlockE (block (T.is_unit note.Note.typ) ds)
  | S.NotE e -> (notE (exp e)).it
  | S.AndE (e1, e2) -> (andE (exp e1) (exp e2)).it
  | S.OrE (e1, e2) -> (orE (exp e1) (exp e2)).it
  | S.IfE (e1, e2, e3) -> I.IfE (exp e1, exp e2, exp e3)
  | S.SwitchE (e1, cs) -> I.SwitchE (exp e1, cases cs)
  | S.TryE (e1, cs, None) -> I.TryE (exp e1, cases cs, None)
  | S.TryE (e1, cs, Some e2) ->
    let thunk = [] -->* exp e2 |> named "$cleanup" in
    assert T.(is_func thunk.note.Note.typ);
    let th = fresh_var "thunk" thunk.note.Note.typ in
    (blockE
       [ letD th thunk ]
       { e1 with it = I.TryE (exp e1, cases cs, Some (id_of_var th, typ_of_var th)); note }).it
  | S.WhileE (e1, e2, flags) ->
    (match desugar_loop_flags at note e2 flags (fun e2 -> S.WhileE (e1, e2, flags)) with
    | `Rec e -> exp' at note e
    | `Body e2 -> (whileE (exp e1) (exp e2)).it)
  | S.LoopE (e1, opt_e2, flags) ->
    (match desugar_loop_flags at note e1 flags (fun e1 -> S.LoopE (e1, opt_e2, flags)) with
    | `Rec e -> exp' at note e
    | `Body e1 -> 
      match opt_e2 with
      | None -> I.LoopE (exp e1)
      | Some e2 -> (loopWhileE (exp e1) (exp e2)).it)
  | S.ForE (p, e1, e2, flags) ->
    (match desugar_loop_flags at note e2 flags (fun e2 -> S.ForE (p, e1, e2, flags)) with
    | `Rec e -> exp' at note e
    | `Body e2 ->
      match e1.it with
      | S.CallE (None, {it=S.DotE (arr, proj, _); _}, _, (_, e1))
        when T.is_array arr.note.S.note_typ && (proj.it = "vals" || proj.it = "values" || proj.it = "keys")
        -> (transform_for_to_while p arr proj (!e1) e2).it
      | _ -> (forE (pat p) (exp e1) (exp e2)).it)
  | S.DebugE e -> if !Mo_config.Flags.release_mode then (unitE ()).it else (exp e).it
  | S.LabelE (l, t, e) -> I.LabelE (l.it, t.Source.note, exp e)
  | S.BreakE (kind, id_opt, e) -> (breakE (S.break_label kind id_opt) (exp e)).it
  | S.RetE e -> (retE (exp e)).it
  | S.ThrowE e -> I.PrimE (I.ThrowPrim, [exp e])
  | S.AsyncE (par_opt, s, tb, e) ->
    let ds, rs = parenthetical (s = T.Fut) par_opt in
    let it = I.AsyncE (s, typ_bind tb, exp e,
                       match note.Note.typ with
                       | T.Async (_, t, _) -> t
                       | _ -> assert false) in
    (blockE (ds @ rs) { at; note; it }).it
  | S.AwaitE (sort, e) -> I.PrimE I.(AwaitPrim sort, [exp e])
  | S.AssertE (Runtime, e) -> I.PrimE (I.AssertPrim, [exp e])
  | S.AnnotE (e, _) -> assert false
  | S.ImportE (f, ir) -> raise (Invalid_argument (Printf.sprintf "Import expression found in unit body: %s" f))
  | S.ImplicitLibE lib -> (varE (var (id_of_full_path lib) note.Note.typ)).it
  | S.PrimE s -> raise (Invalid_argument ("Unapplied prim " ^ s))
  | S.IgnoreE e ->
    I.BlockE ([
      { it = I.LetD ({it = I.WildP; at = e.at; note = T.Any}, exp e);
        at = e.at; note = ()}], (unitE ()))

and parenthetical send = function
  | None -> [], []
  | Some par when not send -> [expD (exp par)], []
  | Some par ->
    (* fishing for relevant attributes in the parenthetical based on its static type *)
    let cycles, clean_cycles =
      if T.(sub par.note.note_typ (Obj (Object, [T.cycles_fld])))
      then [fun parV -> dotE parV T.cycles_lab T.nat |> assignVarE "@cycles" |> expD], []
      else [], [] in
    let timeout, clean_timeout =
      if T.(sub par.note.note_typ (Obj (Object, [T.timeout_fld])))
      then [fun parV -> dotE parV T.timeout_lab T.nat32 |> optE |> assignVarE "@timeout" |> expD], []
      else [], [nullE () |> assignE (var "@timeout" T.(Mut (Opt nat32))) |> expD] in
    (* present attributes need to set variables, absent ones just clear out *)
    let present, absent = cycles @ timeout, clean_cycles @ clean_timeout in
    if present <> [] then
      let parV = fresh_var "par" par.note.note_typ in
      (* for present attributes we evaluate the parenthetical record, and use the binding
         to get the attributes' values from it, then set the backend variables *)
      [letD parV (exp par)], List.map (fun attr -> attr (varE parV)) present @ absent
    (* if all attributes are absent, we still have to evaluate the parenthetical for side-effects *)
    else [expD (exp par)], absent

and url e at =
    (* Set position explicitly *)
    match e.it with
    | S.AnnotE (e,_) -> url e at
    | _ ->
      let e' = exp e in
      { it = I.(PrimE (BlobOfIcUrl, [e'])); at; note = Note.{def with typ = T.blob; eff = e'.note.eff } }

and lexp e =
    (* We short-cut AnnotE here, so that we get the position of the inner expression *)
    match e.it with
    | S.AnnotE (e,_) -> lexp e
    | _ -> { e with it = lexp' e.it; note = e.note.S.note_typ }

and lexp' = function
  | S.VarE i -> I.VarLE i.it
  | S.DotE (e, x, _) -> I.DotLE (exp e, x.it)
  | S.IdxE (e1, e2) -> I.IdxLE (exp e1, exp e2)
  | _ -> raise (Invalid_argument ("Unexpected expression as lvalue"))

and transform_for_to_while p arr_exp proj e1 e2 =
  (* for (p in (arr_exp : [_]).proj(e1)) e2 when proj in {"keys", "vals", "values"}
     ~~>
     let arr = arr_exp ;
     let last = arr.size(e1) : Int - 1 ;
     var indx = 0;
     if (last == -1) { }
     else {
       label l loop {
         let p = arr[indx]; /* sans bound check */
         e2;
         if (indx == last)
         else { break l }
         then { indx += 1 }
       }
     }
  *)
  let arr_typ = arr_exp.note.note_typ in
  let arrv = fresh_var "arr" arr_typ in
  let indx = fresh_var "indx" T.(Mut nat) in
  let indexing_exp = match proj.it with
    | "vals" | "values" -> primE I.DerefArrayOffset [varE arrv; varE indx]
    | "keys" -> varE indx
    | _ -> assert false in
  let last = fresh_var "last" T.int in
  let lab = fresh_id "done" () in
  blockE
    [ letD arrv (exp arr_exp)
    ; expD (exp e1)
    ; letD last (primE I.GetLastArrayOffset [varE arrv]) (* -1 for empty array *)
    ; varD indx (natE Numerics.Nat.zero)]
    (ifE (primE I.EqArrayOffset [varE last; intE (Numerics.Int.of_int (-1))])
      (* empty array, do nothing *)
      (unitE())
      (labelE lab T.unit (
        loopE (
          (blockE
            [ letP (pat p) indexing_exp
            ; expD (exp e2)]
           (ifE (primE I.EqArrayOffset [varE indx; varE last])
             (* last, exit loop *)
             (breakE lab (tupE []))
             (* else increment and continue *)
             (assignE indx (primE I.NextArrayOffset [varE indx]))))))))

and mut m = match m.it with
  | S.Const -> Ir.Const
  | S.Var -> Ir.Var

and obj_block at s exp_opt self_id dfs obj_typ =
  match s.it with
  | T.Object | T.Module ->
    build_obj at s.it self_id dfs obj_typ
  | T.Actor ->
    build_actor at [] exp_opt self_id dfs obj_typ
  | T.Memory | T.Mixin -> assert false

and build_field {T.lab; T.typ;_} =
  { it = I.{ name = lab
           ; var = lab
           }
  ; at = no_region
  ; note = typ
  }

and build_fields obj_typ =
    match obj_typ with
    | T.Obj (_, fields) ->
      (* TBR: do we need to sort val_fields?*)
      List.map build_field (T.val_fields fields)
    | _ -> assert false

and with_self i typ decs =
  let_no_shadow (var i typ) (selfRefE typ) decs

and call_system_func_opt name es obj_typ =
  List.find_map (fun es ->
    match es.it with
    | { S.vis = { it = S.System; _ };
        S.dec = { it = S.LetD( { it = S.VarP id; note; _ }, _, _); at; _ };
        _ }
      when id.it = name ->
      Some (
        match name with
        | "timer" when not !Mo_config.Flags.global_timer -> assert false;
        | "timer" ->
           let timer =
             blockE
               [ expD T.(callE (varE (var id.it note)) [Any]
                   (varE (var "@set_global_timer" T.global_timer_set_type))) ]
               (unitE()) in
           { timer with at }
        | "heartbeat" ->
          blockE
            [ expD (callE (varE (var id.it note)) [T.Any] (unitE())) ]
           (unitE ())
        | "inspect" ->
          let _, tfs = T.as_obj obj_typ in
          let caller = fresh_var "caller" T.caller in
          let arg = fresh_var "arg" T.blob in
          let msg_typ = T.decode_msg_typ tfs in
          let msg = fresh_var "msg" msg_typ in
          let record_typ =
            T.Obj (T.Object, List.sort T.compare_field
             [{T.lab = "caller"; T.typ = typ_of_var caller; T.src = T.empty_src};
               {T.lab = "arg"; T.typ = typ_of_var arg; T.src = T.empty_src};
               {T.lab = "msg"; T.typ = typ_of_var msg; T.src = T.empty_src}])
          in
          let record = fresh_var "record" record_typ in
          let msg_variant =
            switch_textE (primE Ir.ICMethodNamePrim [])
              (List.map (fun tf ->
                (tf.T.lab,
                  match tf.T.typ with
                  | T.Func(T.Local, _, [], [], ts) ->
                    tagE tf.T.lab
                      T.(funcE ("$"^tf.lab) Local Returns [] [] ts
                        (primE (Ir.DeserializePrim ts) [varE arg]))
                  | _ -> assert false))
                (T.as_variant msg_typ))
               (* Trap early, refusing all other messages,
                  including those in T.well_known_actor_fields. *)
              (wildP,
                (primE (Ir.OtherPrim "trap")
                  [textE "canister_inspect_message implicitly refused message"]))
              msg_typ
          in
          let accept = fresh_var "accept" T.bool
          in
            blockE
              [ letD record (
                  blockE [
                    letD caller (primE Ir.ICCallerPrim []);
                    letD arg (primE Ir.ICArgDataPrim []);
                    letD msg msg_variant
                  ]
                  (newObjE T.Object [
                    {it = I.{name = "caller"; var = id_of_var caller}; at = no_region; note = typ_of_var caller };
                    {it = I.{name = "arg"; var = id_of_var arg}; at = no_region; note = typ_of_var arg };
                    {it = I.{name = "msg"; var = id_of_var msg}; at = no_region; note = typ_of_var msg }]
                    record_typ));
                letD accept (callE (varE (var id.it note)) [] (varE record))]
              (ifE (varE accept)
                (unitE ())
                (primE (Ir.OtherPrim "trap")
                  [textE "canister_inspect_message explicitly refused message"]))
        | "lowmemory" ->
          awaitE T.AwaitCmp
            (callE (varE (var id.it note)) [T.scope_bound] (unitE()))
        | name ->
           let inst = match name with
             | "preupgrade" | "postupgrade" -> [T.scope_bound]
             | _ -> [] in
          callE (varE (var id.it note)) inst (tupE []))
    | _ -> None) es
and build_candid ts obj_typ =
  let open Idllib in
  let (args, prog) = Mo_idl.Mo_to_idl.of_service_type ts obj_typ in
  let module WithComments = Arrange_idl.Make(struct let trivia = Some prog.note.Syntax.trivia end) in
  I.{
   args = WithComments.string_of_args args;
   service = WithComments.string_of_prog prog;
  }

and export_footprint self_id expr =
  let open T in
  let {lab;typ;_} = motoko_stable_var_info_fld in
  let v = "$"^lab in
  let size = fresh_var "size" T.nat64 in
  let scope_con1 = Cons.fresh "T1" (Abs ([], scope_bound)) in
  let scope_con2 = Cons.fresh "T2" (Abs ([], Any)) in
  let bind1  = typ_arg scope_con1 Scope scope_bound in
  let bind2 = typ_arg scope_con2 Scope scope_bound in
  let ret_typ = T.Obj(Object,[{lab = "size"; typ = T.nat64; src = empty_src}]) in
  let caller = fresh_var "caller" caller in
  ([ letD (var v typ) (
       funcE v (Shared Query) Promises [bind1] [] [ret_typ] (
           (asyncE T.Fut bind2
              (blockE [
                   letD caller (primE I.ICCallerPrim []);
                   expD (assertE (orE (primE (I.RelPrim (principal, Operator.EqOp))
                                         [varE caller; selfRefE principal])
                                    (primE (I.OtherPrim "is_controller") [varE caller])));
                   letD size (primE (I.ICStableSize expr.note.Note.typ) [expr])
                 ]
                 (newObjE T.Object
                   [{ it = Ir.{name = "size"; var = id_of_var size};
                      at = no_region;
                      note = T.nat64 }]
                   ret_typ))
              (Con (scope_con1, []))))
  )],
  [{ it = I.{ name = lab; var = v }; at = no_region; note = typ }])

and export_runtime_information self_id =
  let open T in
  let {lab;typ;_} = motoko_runtime_information_fld in
  let v = "$"^lab in
  let scope_con1 = Cons.fresh "T1" (Abs ([], scope_bound)) in
  let scope_con2 = Cons.fresh "T2" (Abs ([], Any)) in
  let bind1  = typ_arg scope_con1 Scope scope_bound in
  let bind2 = typ_arg scope_con2 Scope scope_bound in
  let gc_strategy =
    let open Mo_config in
    let strategy = match !Flags.gc_strategy with
    | Flags.Default -> "default"
    | Flags.MarkCompact -> "compacting"
    | Flags.Copying -> "copying"
    | Flags.Generational -> "generational"
    | Flags.Incremental -> "incremental" in
    if !Flags.force_gc then (Printf.sprintf "%s force" strategy) else strategy
  in
  let prim_call function_name = primE (I.OtherPrim function_name) [] in
  let information = [
    ("compilerVersion", textE (Lib.Option.get Source_id.release Source_id.id), T.text);
    ("garbageCollector", textE gc_strategy, T.text);
    ("rtsVersion", prim_call "rts_version", T.text);
    ("sanityChecks", boolE !Mo_config.Flags.sanity, T.bool);
    ("memorySize", prim_call "rts_memory_size", T.nat);
    ("heapSize", prim_call "rts_heap_size", T.nat);
    ("totalAllocation", prim_call "rts_total_allocation", T.nat);
    ("reclaimed", prim_call "rts_reclaimed", T.nat);
    ("maxLiveSize", prim_call "rts_max_live_size", T.nat);
    ("stableMemorySize", prim_call "rts_stable_memory_size", T.nat);
    ("logicalStableMemorySize", prim_call "rts_logical_stable_memory_size", T.nat);
    ("maxStackSize", prim_call "rts_max_stack_size", T.nat);
    ("callbackTableCount", prim_call "rts_callback_table_count", T.nat);
    ("callbackTableSize", prim_call "rts_callback_table_size", T.nat)
  ] in
  let fields = List.map (fun (name, _, typ) -> fresh_var name typ) information in
  (* Use an object return type to allow adding more data in future. *)
  let ret_typ = motoko_runtime_information_type in
  let caller = fresh_var "caller" caller in
  ([ letD (var v typ) (
       funcE v (Shared Query) Promises [bind1] [] [ret_typ] (
           (asyncE T.Fut bind2
              (blockE ([
                  letD caller (primE I.ICCallerPrim []);
                  expD (ifE (orE
                      (primE (I.RelPrim (principal, Operator.EqOp)) [varE caller; selfRefE principal])
                      (primE (I.OtherPrim "is_controller") [varE caller]))
                    (unitE())
                    (primE (Ir.OtherPrim "trap")
                      [textE "Unauthorized call of __motoko_runtime_information"]))
                  ] @
                  (List.map2 (fun field (_, load_info, _) ->
                    letD field load_info
                  ) fields information))
                (newObjE T.Object
                  (List.map2 (fun field (name, _, typ) ->
                      { it = Ir.{name; var = id_of_var field}; at = no_region; note = typ })
                    fields information
                  ) ret_typ))
              (Con (scope_con1, []))))
  )],
  [{ it = I.{ name = lab; var = v }; at = no_region; note = typ }])

and build_stabs (df : S.dec_field) : stab option list = match df.it.S.dec.it with
  | S.TypD _ -> []
  | S.MixinD _ -> assert false
  | S.IncludeD(_, arg, note) ->
    (* TODO: This is ugly. It would be a lot nicer if we didn't have to split
       the desugaring and stability declarations *)
    let flex = Some (S.Flexible @@ no_region) in
    let { imports; decs; _ } = Option.get !note in
    let import_stabs = List.map (fun _ -> flex) imports in
    (* Transient stability for binding the mixin parameters *)
    flex ::
    (* Transient stability for binding the mixin imports *)
    import_stabs @
    List.concat_map build_stabs decs
  | _ -> [df.it.S.stab]

and build_actor at ts (exp_opt : Ir.exp option) self_id es obj_typ =
  let candid = build_candid ts obj_typ in
  let fs = build_fields obj_typ in
  let stabs = List.concat_map build_stabs es in
  let ds = decs (List.map (fun ef -> ef.it.S.dec) es) in
  let pairs = List.map2 stabilize stabs ds in
  let idss = List.map fst pairs in
  let ids = List.concat idss in
  let stab_fields = List.sort T.compare_field
    (List.map (fun (i, t) -> T.{lab = i; typ = t; src = empty_src}) ids)
  in
  let mem_fields =
    List.map
      (fun tf -> {tf with T.typ = T.Opt (T.as_immut tf.T.typ) } )
      stab_fields in
  let mk_ds = List.map snd pairs in
  let mem_ty = T.Obj (T.Memory, mem_fields) in
  let state = fresh_var "state" (T.Mut (T.Opt mem_ty)) in
  let get_state = fresh_var "getState" (T.Func(T.Local, T.Returns, [], [], [mem_ty])) in
  let ds = List.map (fun mk_d -> mk_d get_state) mk_ds in
  let sig_, stable_type, migration = match exp_opt with
    | None ->
      T.Single stab_fields,
      I.{pre = mem_ty; post = mem_ty},
      primE (I.ICStableRead mem_ty) [] (* as before *)
    | Some exp0 ->
      let typ = let _, tfs = T.as_obj_sub [T.migration_lab] exp0.note.Note.typ in
                T.lookup_val_field T.migration_lab tfs
      in
      let e = dotE exp0 T.migration_lab typ in
      let dom, rng = T.as_mono_func_sub typ in
      let (_dom_sort, dom_fields) = T.as_obj (T.normalize dom) in
      let (_rng_sort, rng_fields) = T.as_obj (T.promote rng) in
      let stab_fields_pre =
        List.sort (fun (r1, tf1) (r2, tf2) -> T.compare_field tf1 tf2)
          ((List.map (fun tf -> (true, tf)) dom_fields) (* required *) @
            (List.filter_map
              (fun tf ->
                match T.lookup_val_field_opt tf.T.lab dom_fields,
                      T.lookup_val_field_opt tf.T.lab rng_fields with
                | Some _, _    (* ignore consumed (overridden) *)
                | _, Some _ -> (* ignore produced (provided) *)
                  None
                | None, None ->
                  (* retain others *)
                  Some (false, tf)) (* optional *)
              stab_fields))
      in
      let mem_fields_pre =
        List.map
          (fun (is_required, tf) -> { tf with T.typ = T.Opt (T.as_immut tf.T.typ) })
          stab_fields_pre
      in
      let mem_ty_pre = T.Obj (T.Memory, mem_fields_pre) in
      let v = fresh_var "v" mem_ty_pre in
      let v_dom = fresh_var "v_dom" dom in
      let v_rng = fresh_var "v_rng" rng in
      T.PrePost (stab_fields_pre, stab_fields),
      I.{pre = mem_ty_pre; post = mem_ty},
      ifE (primE (I.OtherPrim "rts_in_upgrade") [])
        (* in upgrade, apply migration *)
        (blockE [
            letD v (primE (I.ICStableRead mem_ty_pre) []);
            letD v_dom
              (objectE T.Object
                (List.map
                  (fun T.{lab=i;typ=t;_} ->
                    let vi = fresh_var ("v_"^i) (T.as_immut t) in
                    (i,
                     switch_optE (dotE (varE v) i (T.Opt (T.as_immut t)))
                       (primE (Ir.OtherPrim "trap")
                         [textE (Printf.sprintf
                           "stable variable `%s` of type `%s` expected but not found"
                           i (T.string_of_typ t))])
                         (varP vi) (varE vi)
                         (T.as_immut t)))
                  dom_fields)
                dom_fields);
            letD v_rng (callE e [] (varE v_dom))
          ]
          (objectE T.Memory
            (List.map
              (fun T.{lab=i;typ=t;_} ->
               i,
               match T.lookup_val_field_opt i rng_fields with
               | Some t -> (* produced by migration *)
                 optE (dotE (varE v_rng) i (T.as_immut t)) (* wrap in ?_*)
               | None -> (* not produced by migration *)
                 match T.lookup_val_field_opt i dom_fields with
                 | Some t ->
                   (* consumed by migration (not produced) *)
                   nullE() (* TBR: could also reuse if compatible *)
                 | None -> dotE (varE v) i t)
              mem_fields)
            mem_fields))
        (* not in upgrade, read record of nulls *)
        (primE (I.ICStableRead mem_ty) [])
  in
  let ds =
    varD state (optE migration)
    ::
    nary_funcD get_state []
      (let v = fresh_var "v" mem_ty in
       switch_optE (immuteE (varE state))
         (unreachableE ())
         (varP v) (varE v)
         mem_ty)
    ::
    ds
    @
    [expD (assignE state (nullE()))]
  in
  let ds' = match self_id with
    | Some n ->
      with_self n.it obj_typ ds
    | None -> ds in
  let meta =
    I.{ candid = candid;
        sig_ = T.string_of_stab_sig sig_} in
  let with_stable_vars wrap =
    let vs = fresh_vars "v" (List.map (fun f -> f.T.typ) mem_fields) in
    blockE
      ((match call_system_func_opt "preupgrade" es obj_typ with
        | Some call -> [ expD call]
        | None -> []) @
         [letP (seqP (List.map varP vs)) (* dereference any mutable vars, option 'em all *)
            (seqE (List.map (fun tf -> optE (varE (var tf.T.lab tf.T.typ))) stab_fields))])
      (wrap
         (newObjE T.Memory
            (List.map2 (fun f v ->
                 { it = I.{name = f.T.lab; var = id_of_var v};
                   at = no_region;
                   note = f.T.typ }
               ) mem_fields vs)
            mem_ty)) in
  let footprint_d, footprint_f = export_footprint self_id (with_stable_vars Fun.id) in
  let runtime_info_d, runtime_info_f = export_runtime_information self_id in
  I.(ActorE (footprint_d @ runtime_info_d @ ds', footprint_f @ runtime_info_f @ fs,
     { meta;
       preupgrade = (primE (I.ICStableWrite mem_ty) []);
       postupgrade =
         (match call_system_func_opt "postupgrade" es obj_typ with
          | Some call -> call
          | None -> tupE []);
       heartbeat =
         (match call_system_func_opt "heartbeat" es obj_typ with
          | Some call -> call
          | None -> tupE []);
       timer =
         (match call_system_func_opt "timer" es obj_typ with
          | Some call -> call
          | None when !Mo_config.Flags.global_timer ->
            blockE
              [ expD T.(callE (varE (var "@timer_helper" T.heartbeat_type)) [unit] (unitE())) ]
              (unitE())
          | None -> tupE []);
       inspect =
         (match call_system_func_opt "inspect" es obj_typ with
          | Some call -> call
          | None -> tupE []);
       low_memory =
         (match call_system_func_opt "lowmemory" es obj_typ with
          | Some call -> call
          | None -> tupE []);
       stable_record = with_stable_vars (fun e -> e);
       stable_type = stable_type
     },
     obj_typ))

and stabilize stab_opt d =
  let s = match stab_opt with None -> S.Flexible | Some s -> s.it  in
  match s, d.it with
  | (S.Flexible, _) ->
    ([], fun _ -> d)
  | (S.Stable, I.VarD(i, t, e)) ->
    ([(i, T.Mut t)],
     fun get_state ->
     let v = fresh_var i t in
     varD (var i (T.Mut t))
       (switch_optE (dotE (callE (varE get_state) [] (unitE ())) i (T.Opt t))
         e
         (varP v) (varE v)
         t))
  | (S.Stable, I.RefD _) -> assert false (* RefD cannot come from user code *)
  | (S.Stable, I.LetD({it = I.VarP i; _} as p, e)) ->
    let t = p.note in
    ([(i, t)],
     fun get_state ->
     let v = fresh_var i t in
     letP p
       (switch_optE (dotE (callE (varE get_state) [] (unitE ())) i (T.Opt t))
         e
         (varP v) (varE v)
         t))
  | (S.Stable, I.LetD _) ->
    assert false

and build_obj at s self_id dfs obj_typ =
  let fs = build_fields obj_typ in
  let obj_e = newObjE s fs obj_typ in
  let ds = decs (List.map (fun df -> df.it.S.dec) dfs) in
  let e = blockE ds obj_e in
  match self_id with
    | None -> e.it
    | Some self_id ->
      let self = var self_id.it obj_typ in
      (letE self e (varE self)).it

and exp_field obj_typ ef =
  let _, fts = T.as_obj_sub [] obj_typ in
  let S.{mut; id; exp = e} = ef.it in
  match mut.it with
  | S.Var ->
    let typ = match T.lookup_val_field_opt id.it fts with
      | Some typ -> typ
      | None -> T.Mut e.note.S.note_typ
    in
    assert (T.is_mut typ);
    let id' = fresh_var id.it typ in
    let d = varD id' (exp e) in
    let f = { it = I.{ name = id.it; var = id_of_var id' }; at = no_region; note = typ } in
    ([d], f)
  | S.Const ->
    let typ = match T.lookup_val_field_opt id.it fts with
      | Some typ -> typ
      | None -> e.note.S.note_typ
    in
    assert (not (T.is_mut typ));
    let e = exp e in
    let id', ds = match e.it with
    | I.(VarE (Const, v)) -> var v typ, []
    | _ -> let id' = fresh_var id.it typ in id', [letD id' e] in
    let f = { it = I.{ name = id.it; var = id_of_var id' }; at = no_region; note = typ } in
    (ds, f)

and obj obj_typ efs bases =
  let open List in
  let base_info base =
    let base_exp, base_t = exp base, (typ_note base.note).Note.typ in
    let base_var = fresh_var "base" base_t in
    let base_dec = letD base_var base_exp in
    let pick l =
      if exists (fun { T.lab; _ } -> lab = l) T.(promote base_t |> as_obj |> snd)
      then [base_var] else [] in
    base_dec, pick in

  let base_decs, pickers = map base_info bases |> split in
  let gap T.{ lab; typ; _ } =
    if exists (fun (ef : S.exp_field) -> ef.it.id.it = lab) efs then []
    else
      let id = fresh_var lab typ in
      let [@warning "-8"] [base_var] = concat_map ((|>) lab) pickers in
      let d =
        if T.is_mut typ then
          refD id { it = I.DotLE(varE base_var, lab); note = typ; at = no_region }
        else
          letD id (dotE (varE base_var) lab typ) in
      let f = { it = I.{ name = lab; var = id_of_var id }; at = no_region; note = typ } in
      [d, f] in

  let dss, fs = map (exp_field obj_typ) efs |> split in
  let ds', fs' = concat_map gap (T.as_obj obj_typ |> snd |> T.val_fields) |> split in
  let obj_e = newObjE T.Object (append fs fs') obj_typ in
  let decs = append base_decs (append (flatten dss) ds') in
  (blockE decs obj_e).it

and typ_binds tbs = List.map typ_bind tbs

and typ_bind tb =
  let c = match tb.note with
    | Some c -> c
    | _ -> assert false
  in
  { it = { Ir.con = c; Ir.sort = tb.it.S.sort.it; Ir.bound = tb.it.S.bound.note}
  ; at = tb.at
  ; note = ()
  }

and array_dotE array_ty proj e =
  let fun_ty bs t1 t2 = T.Func (T.Local, T.Returns, bs, t1, t2) in
  let varA = T.Var ("A", 0) in
  let element_ty = T.as_immut (T.as_array array_ty) in
  let call name t1 t2 =
    let poly_array_ty =
      if T.is_mut (T.as_array array_ty)
      then T.Array (T.Mut varA)
      else T.Array varA in
    let ty_param = {T.var = "A"; sort = T.Type; T.bound = T.Any} in
    let f = var name (fun_ty [ty_param] [poly_array_ty] [fun_ty [] t1 t2]) in
    callE (varE f) [element_ty] e in
  match T.is_mut (T.as_array array_ty), proj with
    | true,  "size" -> call "@mut_array_size"   [] [T.nat]
    | false, "size" -> call "@immut_array_size" [] [T.nat]
    | true,  "get"  -> call "@mut_array_get"    [T.nat] [varA]
    | false, "get"  -> call "@immut_array_get"  [T.nat] [varA]
    | true,  "put"  -> call "@mut_array_put"    [T.nat; varA] []
    | true,  "keys" -> call "@mut_array_keys"   [] T.[iter_obj nat]
    | false, "keys" -> call "@immut_array_keys" [] T.[iter_obj nat]
    | true,  ("vals" | "values") -> call "@mut_array_vals"   [] [T.iter_obj varA]
    | false, ("vals" | "values") -> call "@immut_array_vals" [] [T.iter_obj varA]
    | _, _ -> assert false

and blob_dotE proj e =
  let fun_ty t1 t2 = T.Func (T.Local, T.Returns, [], t1, t2) in
  let call name t1 t2 =
    let f = var name (fun_ty [T.blob] [fun_ty t1 t2]) in
    callE (varE f) [] e in
  match proj with
    | "size" -> call "@blob_size" [] [T.nat]
    | "keys" -> call "@blob_keys" [] T.[iter_obj nat]
    | "vals" | "values" -> call "@blob_vals" [] T.[iter_obj (Prim Nat8)]
    | "get" -> call "@blob_get" [T.nat] T.[Prim Nat8]
    |  _ -> assert false

and text_dotE proj e =
  let fun_ty t1 t2 = T.Func (T.Local, T.Returns, [], t1, t2) in
  let call name t1 t2 =
    let f = var name (fun_ty [T.text] [fun_ty t1 t2]) in
    callE (varE f) [] e in
  match proj with
    | "size"   -> call "@text_size"   [] [T.nat]
    | "chars" -> call "@text_chars" [] [T.iter_obj T.char]
    |  _ -> assert false

and block force_unit ds =
  match ds with
  | [] -> ([], tupE [])
  | [{it = S.ExpD ({it = S.BlockE ds; _}); _}] -> block force_unit ds
  | _ ->
  let prefix, last = Lib.List.split_last ds in
  match force_unit, last.it with
  | _, S.ExpD e ->
    (decs prefix, exp e)
  | false, S.LetD ({it = S.VarP x; _}, e, _) -> (* Fail block dead, pattern match irrefutable *)
    (decs ds, varE (var x.it e.note.S.note_typ))
  | false, S.LetD (p, e, None) ->
    let x = fresh_var "x" (e.note.S.note_typ) in
    (decs prefix @ [letD x (exp e); letP (pat p) (varE x)], varE x)
  | false, S.LetD (p, e, Some f) ->
    (decs prefix, let_else_switch (pat p) (exp e) (exp f))
  | _, _ ->
    (decs ds, tupE [])

and decs ds = List.concat_map dec ds

and dec d = List.map (fun ir_dec -> { it = ir_dec; at = d.at; note = () }) (dec' d)

and dec' d =
  let n = d.note in
  let at = d.at in
  match d.it with
  | S.ExpD e -> [(expD (exp e)).it]
  | S.LetD (p, e, f) ->
    let p' = pat p in
    let e' = exp e in
    (* HACK: remove this once backend supports recursive actors *)
    begin match p'.it, e'.it, f with
    | I.VarP i, I.ActorE (ds, fs, u, t), _ ->
      [I.LetD (p', {e' with it = I.ActorE (with_self i t ds, fs, u, t)})]
    | _, _, None -> [I.LetD (p', e')]
    | _, _, Some f -> [I.LetD (p', let_else_switch (pat p) (exp e) (exp f))]
    end
  | S.VarD (i, e) -> [I.VarD (i.it, e.note.S.note_typ, exp e)]
  | S.TypD _ -> []
  | S.MixinD _ -> []
  | S.IncludeD(_, args, note) ->
    let { imports = is; pat = p; decs } = Option.get !note in
    let ir_imports = List.concat_map transform_import is in
    let renamed_imports, rho = Rename.decs Rename.Renaming.empty ir_imports in
    let renamed_pat, rho = Rename.pat rho (pat p) in

    (* TODO: Fix the positions on the generated let here *)
    let ir_decs = List.concat_map dec (List.map (fun df -> df.it.S.dec) decs) in
    let renamed_decs, _ = Subst_var.decs rho ir_decs in
    List.map (fun d -> d.it) renamed_imports @ (letP renamed_pat (exp args)).it :: List.map (fun d -> d.it) renamed_decs

  | S.ClassD (exp_opt, sp, s, id, tbs, p, _t_opt, self_id, dfs) ->
    let id' = {id with note = ()} in
    let sort, _, _, _, _ = Type.as_func n.S.note_typ in
    let op = match sp.it with
      | T.Local -> None
      | T.Shared (_, p) -> Some p in
    let inst = List.map
                 (fun tb ->
                   match tb.note with
                   | None -> assert false
                   | Some c -> T.Con (c, []))
                 tbs in
    let fun_typ = n.S.note_typ in
    let rng_typ =
      match fun_typ with
      | T.Func(_, _, bds, dom, [rng]) ->
        assert(List.length inst = List.length bds);
        T.promote (T.open_ inst rng)
      | _ -> assert false
    in
    let varPat = {it = I.VarP id'.it; at = at; note = fun_typ } in
    let args, eo, wrap, control, _n_res = to_args n.S.note_typ op exp_opt p in
    let body = if s.it = T.Actor
      then
        let (_, _, obj_typ) = T.as_async rng_typ in
        let c = Cons.fresh T.default_scope_var (T.Abs ([], T.scope_bound)) in
        asyncE T.Fut (typ_arg c T.Scope T.scope_bound) (* TBR *)
          (wrap { it = obj_block at s eo (Some self_id) dfs (T.promote obj_typ);
            at = at;
            note = Note.{def with typ = obj_typ } })
          (List.hd inst)
      else
       wrap
        { it = obj_block at s eo (Some self_id) dfs rng_typ;
          at = at;
          note = Note.{ def with typ = rng_typ } }
    in
    let fn = {
      it = I.FuncE (id.it, sort, control, typ_binds tbs, args, [rng_typ], body);
      at = at;
      note = Note.{ def with typ = fun_typ }
    } in
    [I.LetD (varPat, fn)]

and cases cs = List.map (case Fun.id) cs

and case f c = phrase (case' f) c

and case' f c = S.{ I.pat = pat c.pat; I.exp = f (exp c.exp) }

and pats ps = List.map pat ps

and pat p = phrase pat' p

and pat' = function
  | S.VarP v -> I.VarP v.it
  | S.WildP -> I.WildP
  | S.LitP l -> I.LitP (lit !l)
  | S.SignP (o, l) -> I.LitP (lit (apply_sign o (!l)))
  | S.TupP ps -> I.TupP (pats ps)
  | S.ObjP pfs ->
    I.ObjP (pat_fields pfs)
  | S.OptP p -> I.OptP (pat p)
  | S.TagP (i, p) -> I.TagP (i.it, pat p)
  | S.AltP (p1, p2) -> I.AltP (pat p1, pat p2)
  | S.AnnotP (p, _)
  | S.ParP p -> pat' p.it

and lit = function
  | S.NullLit -> I.NullLit
  | S.BoolLit x -> I.BoolLit x
  | S.NatLit x -> I.NatLit x
  | S.Nat8Lit x -> I.Nat8Lit x
  | S.Nat16Lit x -> I.Nat16Lit x
  | S.Nat32Lit x -> I.Nat32Lit x
  | S.Nat64Lit x -> I.Nat64Lit x
  | S.IntLit x -> I.IntLit x
  | S.Int8Lit x -> I.Int8Lit x
  | S.Int16Lit x -> I.Int16Lit x
  | S.Int32Lit x -> I.Int32Lit x
  | S.Int64Lit x -> I.Int64Lit x
  | S.FloatLit x -> I.FloatLit x
  | S.CharLit x -> I.CharLit x
  | S.TextLit x -> I.TextLit x
  | S.BlobLit x -> I.BlobLit x
  | S.PreLit _ -> assert false

and pat_fields pfs = List.filter_map pat_field pfs
and pat_field pf = match pf.it with
  | S.ValPF(id, p) -> Some { pf with it = I.{name=id.it; pat=pat p} }
  | S.TypPF(_) -> None

and to_args typ po exp_opt p : Ir.arg list * Ir.exp option * (Ir.exp -> Ir.exp) * T.control * T.typ list =

  let mergeE ds e =
    match e.it with
    | Ir.ActorE _ ->
      (match Rename.exp' Rename.Renaming.empty e.it with
       |  Ir.ActorE (ds', fs, up, ot) ->
         { e with it = Ir.ActorE (ds @ ds', fs, up, ot) }
       | _ -> assert false)
    | _ -> blockE ds e
  in

  let sort, control, n_args, res_tys =
    match typ with
    | Type.Func (sort, control, tbds, dom, res) ->
      sort, control, List.length dom, res
    | Type.Non ->
      Type.Local, Type.Returns, 1, []
    | _ -> raise (Invalid_argument ("to_args " ^ Type.string_of_typ typ))
  in

  let tys = if n_args = 1 then [p.note] else T.seq_of_tup p.note in

  let rec pat_unannot p = match p.it with
    | S.AnnotP (p, _) -> pat_unannot p
    | S.ParP p -> pat_unannot p
    | _ -> p
  in

  (*
     In source, the optional shared pattern and migration expression
     are outside the argument pattern, but in the IR, parameters are
     bound first. So if there is either a shared pattern or migration
     expression, we _must_ create fresh names for the parameters and
     bind the actual parameters inside the wrapper.
  *)
  let must_wrap = po <> None || exp_opt <> None in

  let to_arg p : (Ir.arg * (Ir.exp -> Ir.exp)) =
    match (pat_unannot p).it with
    | S.AnnotP _ | S.ParP _ -> assert false
    | S.VarP i when not must_wrap ->
      { i with note = p.note },
      Fun.id
    | S.WildP ->
      let v = fresh_var "param" p.note in
      arg_of_var v,
      Fun.id
    |  _ ->
      let v = fresh_var "param" p.note in
      arg_of_var v,
      (fun e -> mergeE [letP (pat p) (varE v)] e)
  in

  let args, wrap =
    match n_args, (pat_unannot p).it with
    | _, (S.AnnotP _ | S.ParP _) -> assert false
    | _, S.WildP ->
      let vs = fresh_vars "ignored" tys in
      List.map arg_of_var vs,
      Fun.id
    | 1, _ ->
      let a, wrap = to_arg p in
      [a], wrap
    | 0, S.TupP [] ->
      [], Fun.id
    | _, S.TupP ps ->
      assert (List.length ps = n_args);
      List.fold_right (fun p (args, wrap) ->
        let (a, wrap1) = to_arg p in
        (a::args, fun e -> wrap1 (wrap e))
      ) ps ([], Fun.id)
    | _, _ ->
      let vs = fresh_vars "param" tys in
      List.map arg_of_var vs,
      (fun e -> mergeE [letP (pat p) (tupE (List.map varE vs))] e)
  in

  let wrap_po e =
    match po with
    | None -> wrap e
    | Some p ->
      let v = fresh_var "caller" T.caller in
      mergeE
        [letD v (primE I.ICCallerPrim []);
         letP (pat p)
           (newObjE T.Object
              [{ it = Ir.{name = "caller"; var = id_of_var v};
                 at = no_region;
                 note = T.caller }]
              T.ctxt)]
        (wrap e)
  in

  let eo, wrap_exp_opt =
    match exp_opt with
    | None ->
      None,
      fun e -> wrap_po e
    | Some exp0 ->
      let v = fresh_var "migration" exp0.note.S.note_typ in
      Some (varE v),
      fun e ->
        mergeE
        [letD v (exp exp0)]
        (wrap_po e)
  in

  let wrap_under_async e =
    if T.is_shared_sort sort
    then match control, e.it with
      | (T.Promises, Ir.AsyncE (s, tb, e', t)) ->
        { e with it = Ir.AsyncE (s, tb, wrap_exp_opt e', t) }
      | T.Returns, Ir.BlockE (
          [{ it = Ir.LetD ({ it = Ir.WildP; _} as pat, ({ it = Ir.AsyncE (T.Fut, tb,e',t); _} as exp)); _ }],
          ({ it = Ir.PrimE (Ir.TupPrim, []); _} as unit)) ->
        blockE [letP pat {exp with it = Ir.AsyncE (T.Fut, tb, wrap_exp_opt e',t)} ] unit
      | _, Ir.ActorE _ -> wrap_exp_opt e
      | _ -> assert false
    else wrap_exp_opt e
  in
  args, eo, wrap_under_async, control, res_tys

and transform_import (i : S.import) : Ir.dec list =
  let (p, f, ri) = i.it in
  let t = i.note in
  assert (t <> T.Pre);
  match t with
  | T.Obj(T.Mixin, _) -> []
  | _ ->
  let rhs = match !ri with
    | S.Unresolved -> raise (Invalid_argument ("Unresolved import " ^ f))
    | S.LibPath {path = fp; _} ->
      varE (var (id_of_full_path fp) t)
    | S.PrimPath ->
      varE (var (id_of_full_path "@prim") t)
    | S.IDLPath (fp, canister_id) ->
      primE (I.ActorOfIdBlob t) [blobE canister_id]
    | S.ImportedValuePath path ->
       if !Mo_config.Flags.blob_import_placeholders then
         raise (Invalid_argument ("blob import placeholder"))
       else begin
         let contents = Lib.FilePath.contents path in
         assert T.(t = Prim Blob);
         blobE contents
       end
  in [ letP (pat p) rhs ]

type import_declaration = Ir.dec list

let actor_class_mod_exp id class_typ default system =
  let class_con = Cons.fresh id (T.Def([], class_typ)) in
  (objE T.Module
     [(id, class_con)]
     [(id, default);
      ("system", objE T.Module [] [(id, system)])])

let import_compiled_class (lib : S.comp_unit) wasm : import_declaration =
  let f = lib.note.filename in
  let { body; _ } = lib.it in
  let id = match body.it with
    | S.ActorClassU (_, _, _, id, _, _, _, _, _) -> id.it
    | _ -> assert false
  in
  let fun_typ = T.normalize body.note.S.note_typ in
  let s, cntrl, tbs, ts1, ts2 = T.as_func fun_typ in
  let cs = T.open_binds tbs in
  let c, _ = T.as_con (List.hd cs) in
  let ts1' = List.map (T.open_ cs) ts1 in
  let ts2' = List.map (T.open_ cs) ts2 in
  let class_typ = match List.map T.normalize ts2' with
    | [T.Async (_, _, class_typ)] -> class_typ
    | _ -> assert false
  in
  let t_async = T.codom cntrl (fun () -> assert false) ts2' in
  let _, _, t_actor = T.as_async (T.normalize t_async) in
  let cs' = T.open_binds tbs in
  let c', _ = T.as_con (List.hd cs') in
  let install_actor_helper = var "@install_actor_helper"
    T.(Func (Local, Returns, [scope_bind],
      [install_arg_typ; bool; blob; blob],
      [Async(Cmp, Var (default_scope_var, 0), principal)]))
  in
  let wasm_blob = fresh_var "wasm_blob" T.blob in
  let install_arg =
    fresh_var "install_arg" T.install_arg_typ  in
  let system_body install_arg =
    let vs = fresh_vars "param" ts1' in
    let principal = fresh_var "principal" T.principal in
    funcE id T.Local T.Returns
    [typ_arg c T.Scope T.scope_bound]
    (List.map arg_of_var vs)
    ts2'
    (asyncE T.Fut
      (typ_arg c' T.Scope T.scope_bound)
      (letE principal
        (awaitE T.AwaitCmp
          (callE (varE install_actor_helper) cs'
            (tupE [
              install_arg;
              boolE ((!Mo_config.Flags.enhanced_orthogonal_persistence));
              varE wasm_blob;
              primE (Ir.SerializePrim ts1') [seqE (List.map varE vs)]])))
        (primE (Ir.CastPrim (T.principal, t_actor)) [varE principal]))
      (List.hd cs))
  in
  let system = install_arg --> (system_body (varE install_arg)) in
  let default =
    system_body (tagE "new" (recordE ["settings", nullE()]))
  in
  let mod_exp = actor_class_mod_exp id class_typ default system in
  let mod_typ = mod_exp.note.Note.typ in
  [ letD wasm_blob (blobE wasm);
    letD (var (id_of_full_path f) mod_typ) mod_exp ]

let import_prelude prelude : import_declaration =
  decs prelude.it

let inject_decs extra_ds u =
  let open Ir in
  match u with
  | LibU (ds, exp) -> LibU (extra_ds @ ds, exp)
  | ProgU ds -> ProgU (extra_ds @ ds)
  | ActorU (None, ds, fs, up, t) ->
    Ir.ActorU (None, extra_ds @ ds, fs, up, t)
  | ActorU (Some _, _, _, _, _) ->
    let u' = Rename.comp_unit Rename.Renaming.empty u in
    match u' with
    | ActorU (as_opt, ds, fs, up, t) ->
      Ir.ActorU (as_opt, extra_ds @ ds, fs, up, t)
    | _ -> assert false

let link_declarations imports (cu, flavor) =
  inject_decs imports cu, flavor

let transform_unit_body (u : S.comp_unit_body) : Ir.comp_unit =
  match u.it with
  | S.MixinU _ ->
    raise (Invalid_argument "Desugar: Cannot transform mixin compilation unit")
  | S.ProgU ds -> I.ProgU (decs ds)
  | S.ModuleU (self_id, fields) -> (* compiling a module as a library *)
    I.LibU ([], {
      it = build_obj u.at T.Module self_id fields u.note.S.note_typ;
      at = u.at; note = typ_note u.note})
  | S.ActorClassU (_persistence, exp_opt, sp, typ_id, _tbs, p, _, self_id, fields) ->
    let fun_typ = u.note.S.note_typ in
    let op = match sp.it with
      | T.Local -> None
      | T.Shared (_, p) -> Some p in
    let args, eo, wrap, control, _n_res = to_args fun_typ op exp_opt p in
    let (ts, obj_typ) =
      match fun_typ with
      | T.Func(_s, _c, bds, ts1, [async_rng]) ->
        assert(1 = List.length bds);
        let cs  = T.open_binds bds in
        let (_, _, rng) = T.as_async (T.normalize (T.open_ cs async_rng)) in
        List.map (T.open_ cs) ts1,
        T.promote rng
      | _ -> assert false
    in
    let actor_expression = build_actor u.at ts eo (Some self_id) fields obj_typ in
    let e = wrap {
       it = actor_expression;
       at = no_region;
       note = Note.{ def with typ = obj_typ } }
    in
    begin match e.it with
    | I.ActorE(ds, fs, u, t) ->
      I.ActorU (Some args, ds, fs, u, t)
    | _ -> assert false
    end
  | S.ActorU (persistence, exp_opt, self_id, fields) ->
    let eo = Option.map exp exp_opt in
    let ty = u.note.S.note_typ in
    let actor_expression = build_actor u.at [] eo self_id fields ty in
    begin match actor_expression with
    | I.ActorE (ds, fs, u, t) ->
       I.ActorU (None, ds, fs, u, t)
    | _ -> assert false
    end

let transform_unit (u : S.comp_unit) : Ir.prog  =
  let { imports; body; _ } = u.it in
  let imports' = List.concat_map transform_import imports in
  let body' = transform_unit_body body in
  inject_decs imports' body', Ir.full_flavor()


(* Import a unit by composing IR.

   When used for IR interpretation only, this includes
   any class that would have been separately handled by
   import_compiled_class (above) for compilation.
*)
let import_unit (u : S.comp_unit) : import_declaration =
  let { body; _ } = u.it in
  match body.it with
  | S.MixinU _ -> []
  | _ ->
  let f = u.note.filename in
  let t = body.note.S.note_typ in
  assert (t <> T.Pre);
  let prog, _ = transform_unit u in
  match prog with
  | I.LibU (ds, e) ->
    let exp = blockE ds e in
    [ letD (var (id_of_full_path f) exp.note.Note.typ) exp ]
  | I.ActorU (None, ds, fs, up, t) ->
    raise (Invalid_argument "Desugar: Cannot import actor")
  | I.ActorU (Some as_, ds, fs, up, actor_t) ->
    let id = match body.it with
      | S.ActorClassU (persistence, _, _, id, _, _, _, _, _) -> id.it
      | _ -> assert false
    in
    let s, cntrl, tbs, ts1, ts2 = T.as_func t in
    let cs = T.open_binds [T.scope_bind] in
    let c, _ = T.as_con (List.hd cs) in
    let cs' = T.open_binds [T.scope_bind] in
    let c', _ = T.as_con (List.hd cs') in
    let class_typ = match List.map T.normalize ts2 with
      | [ T.Async(_, _, t2) ] -> t2
      | _ -> assert false in
    let install_arg =
      fresh_var "install_arg" T.install_arg_typ
    in
    let system_body install_arg =
      funcE id T.Local T.Returns
        [typ_arg c T.Scope T.scope_bound]
        as_
        [T.Async (T.Fut, List.hd cs, actor_t)]
        (asyncE
          T.Fut
          (typ_arg c' T.Scope T.scope_bound)
          (ifE
            (primE (Ir.RelPrim (T.install_arg_typ, Operator.EqOp))
              [ install_arg;
                tagE "new" (recordE ["settings", nullE()]) ])
             { it = I.ActorE (ds, fs, up, actor_t); at = u.at; note = Note.{ def with typ = actor_t } }
             (primE (Ir.OtherPrim "trap")
               [textE "actor class configuration not supported in interpreter"]))
          (List.hd cs))
    in
    let system = install_arg --> (system_body (varE install_arg)) in
    let system_var = fresh_var "system" system.note.Note.typ in
    let default = (varE system_var) -*- (tagE "new" (recordE ["settings", nullE()])) in
    let mod_exp = actor_class_mod_exp id class_typ default (varE system_var) in
    let mod_typ = mod_exp.note.Note.typ in
    [ letD system_var system;
      letD (var (id_of_full_path f) mod_typ) mod_exp ]
  | I.ProgU ds ->
    raise (Invalid_argument "Desugar: Cannot import program")
