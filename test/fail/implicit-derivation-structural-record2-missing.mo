//MOC-FLAG --package core $MOTOKO_CORE --all-libs

import Order "mo:core/Order";

func compare(__record : [(Text, () -> Order.Order)]) : Order.Order = #equal;

func cmp<R>(x : R, y : R, compare : (implicit : (R, R) -> Order.Order)) : Order.Order =
  compare(x, y);

ignore cmp({ flag = true; n = 1 }, { flag = false; n = 2 });
