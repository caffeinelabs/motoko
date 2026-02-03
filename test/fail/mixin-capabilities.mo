import MixinCap "mixins/MixinCap";
// checks MixinCap can't send but does have system capability
persistent actor {
   include MixinCap(0);
   f<system>();
};
