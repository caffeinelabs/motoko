import MixinCap "mixins/MixinCap";
// checks MixinCap can't send and doesnot have system capability
// Actually, one could allow system capability but fine not to.
actor {
   include MixinCap(0);
   f<system>();
};
