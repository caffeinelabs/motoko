open Type

(*
  Maintain a map from con to info = {Nonproductive, Productive, Param n}`.
  After the analysis, this map says, for all type constructors, whether they are  really unproductive, productive, or if their unfolding is equal to one of their parameters.
* Maintain a set of type definitions under scrutiny
* For each type definition:
  * If it is already in the memo map, use that result. Else analyze it as follows, updating the memo map afterwards.
  * If it is already in the set, we have found a loop. Mark it as `unproductive`. Else add to the setduring the next step.
  * Consider its RHS:
    * If it is a concrete type, return `Productive`.
    * If it is a named type, recurse.
    * If it is the nth type parameter, return `Param n`
    * If it is a type application `T<t1,…,tn>`, recurse.
      * If recursion returns `Productive` or `Nonproductive`, return that.
      * If recursion returns `Param n`, loop to “Consider its RHS”, as if `tn`
        is the RHS.
*)

type info =
  | Nonproductive
  | Productive
  | Param of int

let non_productive cs =
  let map = ref ConEnv.empty in
  let rec rhs cs = function
    | Pre
    | Mut _ ->
      assert false (* body of a Def shouldn't be 2nd class *)
    | Var (s, j) ->
      Param j
    | Con (d, ts) ->
      begin
        visit_con cs d;
        match ConEnv.find d (!map) with
        | Param n ->
          begin
          match Cons.kind d with
          | Def (tbs, t) ->
            assert (n < List.length tbs); (* assume types are arity-correct *)
            rhs cs (List.nth ts n)
          | Abs (tbs, t) ->
            (* we could assert here since Defs should be closed *)
            Productive
          end
        | info -> info
      end
    | Named (n, t) ->
      rhs cs t (* TBR *)
    | _ ->  (* anything else is productive *)
      Productive

  and visit_con cs c =
    match ConEnv.find_opt c (!map) with
    | Some info -> ()
    | None ->
      let info =
        if ConSet.mem c cs then
          Nonproductive
        else
          let t = match Cons.kind c with
            | Def (_, t) -> t
            | _ -> assert false
          in
          rhs (ConSet.add c cs) t
      in
      map := ConEnv.add c info !map
  in
  ConSet.iter (visit_con ConSet.empty) cs;
  ConSet.filter (fun c -> ConEnv.find c !map = Nonproductive) cs
