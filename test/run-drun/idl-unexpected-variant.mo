import Prim "mo:prim";

actor This {

  public type CreateServiceNervousSystem = { somefield: Nat };
  public type Action = {
    #RegisterKnownNeuron : Nat;
    #ManageNeuron;
    #CreateServiceNervousSystem : CreateServiceNervousSystem;
    #ExecuteNnsFunction;
    #RewardNodeProvider;
    #OpenSnsTokenSwap;
    #SetSnsTokenSwapOpenTimeWindow;
    #SetDefaultFollowees;
    #RewardNodeProviders;
    #ManageNetworkEconomics;
    #ApproveGenesisKyc;
    #AddOrRemoveNodeProvider;
    #Motion;
  };
  public type ListProposalInfoResponse = { proposal_info : [ProposalInfo] };
  public type Proposal = {
    url : Text;
    title : ?Text;
    action : ?Action;
    summary : Text;
  };
  public type ProposalInfo = {
    proposal : ?Proposal;
  };
  public type Service =  actor {
    list_proposals : shared query () -> async ListProposalInfoResponse;
  };

  public type Action_ = {
    #RegisterKnownNeuron : Nat;
    #ManageNeuron;
    #ExecuteNnsFunction;
    #RewardNodeProvider;
    #OpenSnsTokenSwap;
    #SetSnsTokenSwapOpenTimeWindow;
    #SetDefaultFollowees;
    #RewardNodeProviders;
    #ManageNetworkEconomics;
    #ApproveGenesisKyc;
    #AddOrRemoveNodeProvider;
    #Motion;
  };
  public type ListProposalInfoResponse_ = { proposal_info : [ProposalInfo_] };
  public type Proposal_ = {
    url : Text;
    title : ?Text;
    action : ?Action_;
    summary : Text;
  };
  public type ProposalInfo_ = {
    proposal : ?Proposal_;
  };
  public type Service_ =  actor {
    list_proposals : shared query () -> async ListProposalInfoResponse_;
  };

  public shared query func list_proposals() : async ListProposalInfoResponse {
    {
      proposal_info = [
        {
          proposal = ?
            {
              url = "abc";
              title = ?"abc";
              action = ?#CreateServiceNervousSystem{ somefield=0 };
              summary = "abc";
            }
        },
        {
          proposal = ?
            {
              url = "abc";
              title = ?"abc";
              action = ?(#RegisterKnownNeuron 1);
              summary = "abc";
            }
        }
      ]
    }
  };

  // Say the given phase.
  public func go() : async () {
    let That = actor (debug_show Prim.principalOfActor(This)) : Service_;
    let t = await That.list_proposals();
    Prim.debugPrint (debug_show t)
  };

};

//CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low
