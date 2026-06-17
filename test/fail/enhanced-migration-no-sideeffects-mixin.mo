//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test1
//MOC-FLAG -A=M0194
//MOC-FLAG --package core $MOTOKO_CORE

import Mixin "mixins/NoSideEffect";

actor {
  include Mixin();
};
