//MOC-FLAG --actor-idl issue-2529
//MOC-FLAG --actor-alias qr lg264-qjkae

import QR "canister:qr";

actor {

  type ErrorCorrection = QR.ErrorCorrection;
  type Mode = QR.Mode;
  type Version = QR.Version;

  public func encode(
    version : Version,
    level : ErrorCorrection,
    mode : Mode,
    text : Text,
  ) : async Text {
    let result = await QR.encode(version, level, mode, text);
    switch result {
      case ?matrix { "\n" # (await QR.show(matrix)) }
      case _ { "Error: Invalid input!" }
    }
  };
}

/*
produces bad IDL:
[nix-shell:~/motoko/test/fail]$ moc --idl --actor-idl issue-2529 --actor-alias qr rkp4c-7iaaa-aaaaa-aaaca-cai issue-2529.mo

[nix-shell:~/motoko/test/fail]$ more issue-2529.did 
type Version_2 = Version_2; // circular!
type Version = Version_2;
type Mode_2 = Mode_2; // circular!
type Mode = Mode_2;
type ErrorCorrection_2 = ErrorCorrection_2; // circular!
type ErrorCorrection = ErrorCorrection_2;
service : {
  encode: (Version, ErrorCorrection, Mode, text) -> (text);
}

*/
