//MOC-FLAG --generate-view-queries --package core $MOTOKO_CORE
import Map "mo:core/Map";
import Set "mo:core/Set";
import Text "mo:core/Text";
import Principal "mo:core/Principal";
import Debug "mo:core/Debug";

// mixin providing views for Core data structures
import Views "data-view-sample/views";

/*
produces .did

service : {
  __customers: () -> (reserved) query; // approximated
  go: () -> ();
}
*/

persistent actor Self {

  include Views();

  // ── Customers ──────────────────────────────────────────────────

  let customers : Map.Map<Text, {
    companyName : Text;
    contactName : Text;
    contactTitle : Text;
    city : Text;
    countries : Set.Set<Text>;
  }> = Map.empty();

  customers.add("ALFKI", {companyName = "Alfreds Futterkiste"; contactName = "Maria Anders"; contactTitle = "Sales Representative"; city = "Berlin"; countries = Set.singleton "Germany"});
  customers.add("ANATR", {companyName = "Ana Trujillo Emparedados y helados"; contactName = "Ana Trujillo"; contactTitle = "Owner"; city = "Mexico D.F."; countries = Set.singleton "Mexico"});
  customers.add("ANTON", {companyName = "Antonio Moreno Taqueria"; contactName = "Antonio Moreno"; contactTitle = "Owner"; city = "Mexico D.F."; countries = Set.singleton "Mexico"});
  customers.add("AROUT", {companyName = "Around the Horn"; contactName = "Thomas Hardy"; contactTitle = "Sales Representative"; city = "London"; countries = Set.singleton "UK"});
  customers.add("BERGS", {companyName = "Berglunds snabbkop"; contactName = "Christina Berglund"; contactTitle = "Order Administrator"; city = "Lulea"; countries = Set.singleton "Sweden"});
  customers.add("BLAUS", {companyName = "Blauer See Delikatessen"; contactName = "Hanna Moos"; contactTitle = "Sales Representative"; city = "Mannheim"; countries = Set.singleton "Germany"});
  customers.add("BLONP", {companyName = "Blondesddsl pere et fils"; contactName = "Frederique Citeaux"; contactTitle = "Marketing Manager"; city = "Strasbourg"; countries = Set.singleton "France"});
  customers.add("BOLID", {companyName = "Bolido Comidas preparadas"; contactName = "Martin Sommer"; contactTitle = "Owner"; city = "Madrid"; countries = Set.singleton "Spain"});
  customers.add("BONAP", {companyName = "Bon app'"; contactName = "Laurence Lebihan"; contactTitle = "Owner"; city = "Marseille"; countries = Set.singleton "France"});
  customers.add("BOTTM", {companyName = "Bottom-Dollar Markets"; contactName = "Elizabeth Lincoln"; contactTitle = "Accounting Manager"; city = "Tsawassen"; countries = Set.singleton "Canada"});
  customers.add("BSBEV", {companyName = "B's Beverages"; contactName = "Victoria Ashworth"; contactTitle = "Sales Representative"; city = "London"; countries = Set.singleton "UK"});
  customers.add("CACTU", {companyName = "Cactus Comidas para llevar"; contactName = "Patricio Simpson"; contactTitle = "Sales Agent"; city = "Buenos Aires"; countries = Set.singleton "Argentina"});
  customers.add("CENTC", {companyName = "Centro comercial Moctezuma"; contactName = "Francisco Chang"; contactTitle = "Marketing Manager"; city = "Mexico D.F."; countries = Set.singleton "Mexico"});
  customers.add("CHOPS", {companyName = "Chop-suey Chinese"; contactName = "Yang Wang"; contactTitle = "Owner"; city = "Bern"; countries = Set.singleton "Switzerland"});
  customers.add("COMMI", {companyName = "Comercio Mineiro"; contactName = "Pedro Afonso"; contactTitle = "Sales Associate"; city = "Sao Paulo"; countries = Set.singleton "Brazil"});

  func debugShow(_ : Any) { Debug.print("any") };

  public func go() : async () {
    let views = actor (debug_show (Principal.fromActor(Self))) :
      actor {
        __customers: query () ->
	  async Any
     };

     debugShow(await views.__customers());
   }

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"



