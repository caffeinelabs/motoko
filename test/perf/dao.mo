//MOC-FLAG -A=M0194
import Trie "assetstorage/Trie";
import Principal "assetstorage/Principal";
import Option "assetstorage/Option";
import Iter "assetstorage/Iter";
import Nat "assetstorage/Nat";
import Result "assetstorage/Result";
import Error "assetstorage/Error";
import ICRaw "assetstorage/ExperimentalInternetComputer";
import List "assetstorage/List";
//import Time "assetstorage/Time";
import Types "dao/dao-types";
import Prim "mo:⛔";

shared ({ caller = creator }) actor class DAO() = Self {
  let nominal = { amount_e8s = 30000 };
  let init : Types.BasicDaoStableStorage = {
    accounts = [{ owner = creator; tokens = { amount_e8s = 3000000 } }];
    proposals = [];
    system_params = {
      transfer_fee = nominal;
      proposal_vote_threshold = nominal;
      proposal_submission_deposit = nominal;
    };
  };
  stable var accounts = Types.accounts_fromArray(init.accounts);
  stable var proposals = Types.proposals_fromArray(init.proposals);
  stable var next_proposal_id : Nat = 0;
  stable var system_params : Types.SystemParams = init.system_params;

  // Disabled because it leads to non-deterministic performance measurements.
  // system func heartbeat() : async () {
  //     await execute_accepted_proposals();
  // };

  func account_get(id : Principal) : ?Types.Tokens = Trie.get(accounts, Types.account_key(id), Principal.equal);
  func account_put(id : Principal, tokens : Types.Tokens) {
    accounts := Trie.put(accounts, Types.account_key(id), Principal.equal, tokens).0;
  };
  func proposal_get(id : Nat) : ?Types.Proposal = Trie.get(proposals, Types.proposal_key(id), Nat.equal);
  func proposal_put(id : Nat, proposal : Types.Proposal) {
    proposals := Trie.put(proposals, Types.proposal_key(id), Nat.equal, proposal).0;
  };

  /// Transfer tokens from the caller's account to another account
  public shared ({ caller }) func transfer(transfer : Types.TransferArgs) : async Types.Result<(), Text> {
    switch (account_get caller) {
      case null { #err "Caller needs an account to transfer funds" };
      case (?from_tokens) {
        let fee = system_params.transfer_fee.amount_e8s;
        let amount = transfer.amount.amount_e8s;
        if (from_tokens.amount_e8s < amount + fee) {
          #err("Caller's account has insufficient funds to transfer " # debug_show (amount));
        } else {
          let from_amount : Nat = from_tokens.amount_e8s - amount - fee;
          account_put(caller, { amount_e8s = from_amount });
          let to_amount = Option.get(account_get(transfer.to), Types.zeroToken).amount_e8s + amount;
          account_put(transfer.to, { amount_e8s = to_amount });
          #ok;
        };
      };
    };
  };

  /// Return the account balance of the caller
  public query ({ caller }) func account_balance() : async Types.Tokens {
    Option.get(account_get(caller), Types.zeroToken);
  };

  /// Lists all accounts
  public query func list_accounts() : async [Types.Account] {
    Iter.toArray(
      Iter.map(
        Trie.iter(accounts),
        func((owner : Principal, tokens : Types.Tokens)) : Types.Account = {
          owner;
          tokens;
        },
      )
    );
  };

  /// Submit a proposal
  ///
  /// A proposal contains a canister ID, method name and method args. If enough users
  /// vote "yes" on the proposal, the given method will be called with the given method
  /// args on the given canister.
  public shared ({ caller }) func submit_proposal(payload : Types.ProposalPayload) : async Types.Result<Nat, Text> {
    Result.chain(
      deduct_proposal_submission_deposit(caller),
      func(()) : Types.Result<Nat, Text> {
        let proposal_id = next_proposal_id;
        next_proposal_id += 1;

        let proposal : Types.Proposal = {
          id = proposal_id;
          timestamp = 42424242; //Time.now();
          proposer = caller;
          payload;
          state = #open;
          votes_yes = Types.zeroToken;
          votes_no = Types.zeroToken;
          voters = List.nil();
        };
        proposal_put(proposal_id, proposal);
        #ok(proposal_id);
      },
    );
  };

  /// Return the proposal with the given ID, if one exists
  public query func get_proposal(proposal_id : Nat) : async ?Types.Proposal {
    proposal_get(proposal_id);
  };

  /// Return the list of all proposals
  public query func list_proposals() : async [Types.Proposal] {
    Iter.toArray(Iter.map(Trie.iter(proposals), func(kv : (Nat, Types.Proposal)) : Types.Proposal = kv.1));
  };

  // Vote on an open proposal
  public shared ({ caller }) func vote(args : Types.VoteArgs) : async Types.Result<Types.ProposalState, Text> {
    switch (proposal_get(args.proposal_id)) {
      case null {
        #err("No proposal with ID " # debug_show (args.proposal_id) # " exists");
      };
      case (?proposal) {
        var state = proposal.state;
        if (state != #open) {
          return #err("Proposal " # debug_show (args.proposal_id) # " is not open for voting");
        };
        switch (account_get(caller)) {
          case null {
            return #err("Caller does not have any tokens to vote with");
          };
          case (?{ amount_e8s = voting_tokens }) {
            if (List.some(proposal.voters, func(e : Principal) : Bool = e == caller)) {
              return #err("Already voted");
            };

            var votes_yes = proposal.votes_yes.amount_e8s;
            var votes_no = proposal.votes_no.amount_e8s;
            switch (args.vote) {
              case (#yes) { votes_yes += voting_tokens };
              case (#no) { votes_no += voting_tokens };
            };
            let voters = List.push(caller, proposal.voters);

            if (votes_yes >= system_params.proposal_vote_threshold.amount_e8s) {
              // Refund the proposal deposit when the proposal is accepted
              ignore do ? {
                let account = account_get(proposal.proposer)!;
                let refunded = account.amount_e8s + system_params.proposal_submission_deposit.amount_e8s;
                account_put(proposal.proposer, { amount_e8s = refunded });
              };
              state := #accepted;
            };

            if (votes_no >= system_params.proposal_vote_threshold.amount_e8s) {
              state := #rejected;
            };

            let updated_proposal = {
              id = proposal.id;
              votes_yes = { amount_e8s = votes_yes };
              votes_no = { amount_e8s = votes_no };
              voters;
              state;
              timestamp = proposal.timestamp;
              proposer = proposal.proposer;
              payload = proposal.payload;
            };
            proposal_put(args.proposal_id, updated_proposal);
          };
        };
        #ok(state);
      };
    };
  };

  /// Get the current system params
  public query func get_system_params() : async Types.SystemParams {
    system_params;
  };

  /// Update system params
  ///
  /// Only callable via proposal execution
  public shared ({ caller }) func update_system_params(payload : Types.UpdateSystemParamsPayload) : async () {
    if (caller != Principal.fromActor(Self)) {
      return;
    };
    system_params := {
      transfer_fee = Option.get(payload.transfer_fee, system_params.transfer_fee);
      proposal_vote_threshold = Option.get(payload.proposal_vote_threshold, system_params.proposal_vote_threshold);
      proposal_submission_deposit = Option.get(payload.proposal_submission_deposit, system_params.proposal_submission_deposit);
    };
  };

  /// Deduct the proposal submission deposit from the caller's account
  func deduct_proposal_submission_deposit(caller : Principal) : Types.Result<(), Text> {
    switch (account_get(caller)) {
      case null { #err "Caller needs an account to submit a proposal" };
      case (?from_tokens) {
        let threshold = system_params.proposal_submission_deposit.amount_e8s;
        if (from_tokens.amount_e8s < threshold) {
          #err("Caller's account must have at least " # debug_show (threshold) # " to submit a proposal");
        } else {
          let from_amount : Nat = from_tokens.amount_e8s - threshold;
          account_put(caller, { amount_e8s = from_amount });
          #ok;
        };
      };
    };
  };

  /// Execute all accepted proposals
  public func execute_accepted_proposals() : async () {
    let accepted_proposals = Trie.filter(proposals, func(_ : Nat, proposal : Types.Proposal) : Bool = proposal.state == #accepted);
    // Update proposal state, so that it won't be picked up by the next heartbeat
    for ((id, proposal) in Trie.iter(accepted_proposals)) {
      update_proposal_state(proposal, #executing);
    };

    for ((id, proposal) in Trie.iter(accepted_proposals)) {
      switch (await execute_proposal(proposal)) {
        case (#ok) { update_proposal_state(proposal, #succeeded) };
        case (#err(err)) { update_proposal_state(proposal, #failed(err)) };
      };
    };
  };

  /// Execute the given proposal
  func execute_proposal(proposal : Types.Proposal) : async Types.Result<(), Text> {
    try {
      let payload = proposal.payload;
      ignore await ICRaw.call(payload.canister_id, payload.method, payload.message);
      #ok;
    } catch (e) { #err(Error.message e) };
  };

  func update_proposal_state(proposal : Types.Proposal, state : Types.ProposalState) {
    let updated = {
      state;
      id = proposal.id;
      votes_yes = proposal.votes_yes;
      votes_no = proposal.votes_no;
      voters = proposal.voters;
      timestamp = proposal.timestamp;
      proposer = proposal.proposer;
      payload = proposal.payload;
    };
    proposal_put(proposal.id, updated);
  };

  // Below methods just for the sake of candid generation...
  public query func sample_dummy_proposal() : async (payload : Types.ProposalPayload) {
    {
      method = "dummy";
      canister_id = Principal.fromActor Self;
      message = "" : Blob;
    };
  };

  public query func sample_transfer_proposal() : async (payload : Types.ProposalPayload) {
    {
      method = "transfer";
      canister_id = Principal.fromActor Self;
      message = to_candid ({ amount = nominal; to = Principal.fromActor Self } : Types.TransferArgs);
    };
  };

  public query func sample_vote() : async (payload : Types.VoteArgs) {
    { proposal_id = 1; vote = #yes };
  };
  public func getPerfData() : async () {
    Prim.debugPrint("instructions: " # debug_show (Prim.rts_lifetime_instructions()));
  };
};

//CALL query list_proposals 0x4449444C0000
//CALL query sample_dummy_proposal 0x4449444C0000
//CALL ingress submit_proposal 0x4449444c026c03e1edeb4a71b3c4b1f20468c7ebc4d009016d7b01000564756d6d79010a0000000000000000010100
//CALL query account_balance 0x4449444C0000
//CALL query sample_transfer_proposal 0x4449444C0000
//CALL ingress submit_proposal 0x4449444c026c03e1edeb4a71b3c4b1f20468c7ebc4d009016d7b0100087472616e73666572010a000000000000000001012a4449444c026c02fbca0168d8a38ca80d016c01b9ef9380087d0100010a00000000000000000101b0ea01
//CALL query list_proposals 0x4449444C0000
//CALL query list_accounts 0x4449444C0000
//CALL ingress execute_accepted_proposals 0x4449444C0000

//CALL query get_proposal 0x4449444c00017d00
//CALL query get_proposal 0x4449444c00017d01
//CALL query get_proposal 0x4449444c00017d0f

//CALL query sample_vote 0x4449444C0000
//CALL ingress vote 0x4449444c026c02ea99cff204018882e69c0a7d6b02c1c0017fa7d2f0027f01000101
//CALL ingress vote 0x4449444c026c02ea99cff204018882e69c0a7d6b02c1c0017fa7d2f0027f01000101
//CALL query get_proposal 0x4449444c00017d01
//CALL ingress execute_accepted_proposals 0x4449444C0000

//CALL ingress getPerfData 0x4449444C0000
