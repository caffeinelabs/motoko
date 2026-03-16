//MOC-FLAG --generate-view-queries --package core $MOTOKO_CORE
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Nat32 "mo:core/Nat32";
import Char "mo:core/Char";
import Text "mo:core/Text";
import Principal "mo:core/Principal";
import Debug "mo:core/Debug";

// mixin providing views for Core data structures
import Views "data-view-sample/views";

persistent actor Self {

  include Views();

  // ═══════════════════════════════════════════════════════════════
  // Northwind Database — classic Microsoft sample data
  // ═══════════════════════════════════════════════════════════════

  // ── Categories ─────────────────────────────────────────────────

  let categories : Map.Map<Nat, {
    name : Text;
    description : Text;
  }> = Map.empty();

  categories.add(1, {name = "Beverages"; description = "Soft drinks, coffees, teas, beers, and ales"});
  categories.add(2, {name = "Condiments"; description = "Sweet and savory sauces, relishes, spreads, and seasonings"});
  categories.add(3, {name = "Confections"; description = "Desserts, candies, and sweet breads"});
  categories.add(4, {name = "Dairy Products"; description = "Cheeses"});
  categories.add(5, {name = "Grains/Cereals"; description = "Breads, crackers, pasta, and cereal"});
  categories.add(6, {name = "Meat/Poultry"; description = "Prepared meats"});
  categories.add(7, {name = "Produce"; description = "Dried fruit and bean curd"});
  categories.add(8, {name = "Seafood"; description = "Seaweed and fish"});

  // ── Suppliers ──────────────────────────────────────────────────

  let suppliers : Map.Map<Nat, {
    companyName : Text;
    contactName : Text;
    city : Text;
    country : Text;
  }> = Map.empty();

  suppliers.add(1, {companyName = "Exotic Liquids"; contactName = "Charlotte Cooper"; city = "London"; country = "UK"});
  suppliers.add(2, {companyName = "New Orleans Cajun Delights"; contactName = "Shelley Burke"; city = "New Orleans"; country = "USA"});
  suppliers.add(3, {companyName = "Grandma Kelly's Homestead"; contactName = "Regina Murphy"; city = "Ann Arbor"; country = "USA"});
  suppliers.add(4, {companyName = "Tokyo Traders"; contactName = "Yoshi Nagase"; city = "Tokyo"; country = "Japan"});
  suppliers.add(5, {companyName = "Cooperativa de Quesos Las Cabras"; contactName = "Antonio del Valle Saavedra"; city = "Oviedo"; country = "Spain"});
  suppliers.add(6, {companyName = "Mayumi's"; contactName = "Mayumi Ohno"; city = "Osaka"; country = "Japan"});
  suppliers.add(7, {companyName = "Pavlova Ltd."; contactName = "Ian Devling"; city = "Melbourne"; country = "Australia"});
  suppliers.add(8, {companyName = "Specialty Biscuits Ltd."; contactName = "Peter Wilson"; city = "Manchester"; country = "UK"});
  suppliers.add(9, {companyName = "PB Knackebrod AB"; contactName = "Lars Peterson"; city = "Goteborg"; country = "Sweden"});
  suppliers.add(10, {companyName = "Refrescos Americanas LTDA"; contactName = "Carlos Diaz"; city = "Sao Paulo"; country = "Brazil"});

  // ── Employees ──────────────────────────────────────────────────

  let employees : Map.Map<Nat, {
    lastName : Text;
    firstName : Text;
    title : Text;
    city : Text;
    country : Text;
  }> = Map.empty();

  employees.add(1, {lastName = "Davolio"; firstName = "Nancy"; title = "Sales Representative"; city = "Seattle"; country = "USA"});
  employees.add(2, {lastName = "Fuller"; firstName = "Andrew"; title = "Vice President Sales"; city = "Tacoma"; country = "USA"});
  employees.add(3, {lastName = "Leverling"; firstName = "Janet"; title = "Sales Representative"; city = "Kirkland"; country = "USA"});
  employees.add(4, {lastName = "Peacock"; firstName = "Margaret"; title = "Sales Representative"; city = "Redmond"; country = "USA"});
  employees.add(5, {lastName = "Buchanan"; firstName = "Steven"; title = "Sales Manager"; city = "London"; country = "UK"});
  employees.add(6, {lastName = "Suyama"; firstName = "Michael"; title = "Sales Representative"; city = "London"; country = "UK"});
  employees.add(7, {lastName = "King"; firstName = "Robert"; title = "Sales Representative"; city = "London"; country = "UK"});
  employees.add(8, {lastName = "Callahan"; firstName = "Laura"; title = "Inside Sales Coordinator"; city = "Seattle"; country = "USA"});
  employees.add(9, {lastName = "Dodsworth"; firstName = "Anne"; title = "Sales Representative"; city = "London"; country = "UK"});

  // ── Customers ──────────────────────────────────────────────────

  let customers : Map.Map<Text, {
    companyName : Text;
    contactName : Text;
    contactTitle : Text;
    city : Text;
    country : Text;
  }> = Map.empty();

  customers.add("ALFKI", {companyName = "Alfreds Futterkiste"; contactName = "Maria Anders"; contactTitle = "Sales Representative"; city = "Berlin"; country = "Germany"});
  customers.add("ANATR", {companyName = "Ana Trujillo Emparedados y helados"; contactName = "Ana Trujillo"; contactTitle = "Owner"; city = "Mexico D.F."; country = "Mexico"});
  customers.add("ANTON", {companyName = "Antonio Moreno Taqueria"; contactName = "Antonio Moreno"; contactTitle = "Owner"; city = "Mexico D.F."; country = "Mexico"});
  customers.add("AROUT", {companyName = "Around the Horn"; contactName = "Thomas Hardy"; contactTitle = "Sales Representative"; city = "London"; country = "UK"});
  customers.add("BERGS", {companyName = "Berglunds snabbkop"; contactName = "Christina Berglund"; contactTitle = "Order Administrator"; city = "Lulea"; country = "Sweden"});
  customers.add("BLAUS", {companyName = "Blauer See Delikatessen"; contactName = "Hanna Moos"; contactTitle = "Sales Representative"; city = "Mannheim"; country = "Germany"});
  customers.add("BLONP", {companyName = "Blondesddsl pere et fils"; contactName = "Frederique Citeaux"; contactTitle = "Marketing Manager"; city = "Strasbourg"; country = "France"});
  customers.add("BOLID", {companyName = "Bolido Comidas preparadas"; contactName = "Martin Sommer"; contactTitle = "Owner"; city = "Madrid"; country = "Spain"});
  customers.add("BONAP", {companyName = "Bon app'"; contactName = "Laurence Lebihan"; contactTitle = "Owner"; city = "Marseille"; country = "France"});
  customers.add("BOTTM", {companyName = "Bottom-Dollar Markets"; contactName = "Elizabeth Lincoln"; contactTitle = "Accounting Manager"; city = "Tsawassen"; country = "Canada"});
  customers.add("BSBEV", {companyName = "B's Beverages"; contactName = "Victoria Ashworth"; contactTitle = "Sales Representative"; city = "London"; country = "UK"});
  customers.add("CACTU", {companyName = "Cactus Comidas para llevar"; contactName = "Patricio Simpson"; contactTitle = "Sales Agent"; city = "Buenos Aires"; country = "Argentina"});
  customers.add("CENTC", {companyName = "Centro comercial Moctezuma"; contactName = "Francisco Chang"; contactTitle = "Marketing Manager"; city = "Mexico D.F."; country = "Mexico"});
  customers.add("CHOPS", {companyName = "Chop-suey Chinese"; contactName = "Yang Wang"; contactTitle = "Owner"; city = "Bern"; country = "Switzerland"});
  customers.add("COMMI", {companyName = "Comercio Mineiro"; contactName = "Pedro Afonso"; contactTitle = "Sales Associate"; city = "Sao Paulo"; country = "Brazil"});

  // ── Products ───────────────────────────────────────────────────

  let products : Map.Map<Nat, {
    name : Text;
    supplierId : Nat;
    categoryId : Nat;
    unitPrice : Nat;
    unitsInStock : Nat;
    discontinued : Bool;
  }> = Map.empty();

  products.add(1,  {name = "Chai"; supplierId = 1; categoryId = 1; unitPrice = 18; unitsInStock = 39; discontinued = false});
  products.add(2,  {name = "Chang"; supplierId = 1; categoryId = 1; unitPrice = 19; unitsInStock = 17; discontinued = false});
  products.add(3,  {name = "Aniseed Syrup"; supplierId = 1; categoryId = 2; unitPrice = 10; unitsInStock = 13; discontinued = false});
  products.add(4,  {name = "Chef Anton's Cajun Seasoning"; supplierId = 2; categoryId = 2; unitPrice = 22; unitsInStock = 53; discontinued = false});
  products.add(5,  {name = "Chef Anton's Gumbo Mix"; supplierId = 2; categoryId = 2; unitPrice = 21; unitsInStock = 0; discontinued = true});
  products.add(6,  {name = "Grandma's Boysenberry Spread"; supplierId = 3; categoryId = 2; unitPrice = 25; unitsInStock = 120; discontinued = false});
  products.add(7,  {name = "Uncle Bob's Organic Dried Pears"; supplierId = 3; categoryId = 7; unitPrice = 30; unitsInStock = 15; discontinued = false});
  products.add(8,  {name = "Northwoods Cranberry Sauce"; supplierId = 3; categoryId = 2; unitPrice = 40; unitsInStock = 6; discontinued = false});
  products.add(9,  {name = "Mishi Kobe Niku"; supplierId = 4; categoryId = 6; unitPrice = 97; unitsInStock = 29; discontinued = true});
  products.add(10, {name = "Ikura"; supplierId = 4; categoryId = 8; unitPrice = 31; unitsInStock = 31; discontinued = false});
  products.add(11, {name = "Queso Cabrales"; supplierId = 5; categoryId = 4; unitPrice = 21; unitsInStock = 22; discontinued = false});
  products.add(12, {name = "Queso Manchego La Pastora"; supplierId = 5; categoryId = 4; unitPrice = 38; unitsInStock = 86; discontinued = false});
  products.add(13, {name = "Konbu"; supplierId = 6; categoryId = 8; unitPrice = 6; unitsInStock = 24; discontinued = false});
  products.add(14, {name = "Tofu"; supplierId = 6; categoryId = 7; unitPrice = 23; unitsInStock = 35; discontinued = false});
  products.add(15, {name = "Genen Shouyu"; supplierId = 6; categoryId = 2; unitPrice = 16; unitsInStock = 39; discontinued = false});
  products.add(16, {name = "Pavlova"; supplierId = 7; categoryId = 3; unitPrice = 17; unitsInStock = 29; discontinued = false});
  products.add(17, {name = "Alice Mutton"; supplierId = 7; categoryId = 6; unitPrice = 39; unitsInStock = 0; discontinued = true});
  products.add(18, {name = "Carnarvon Tigers"; supplierId = 7; categoryId = 8; unitPrice = 62; unitsInStock = 42; discontinued = false});
  products.add(19, {name = "Teatime Chocolate Biscuits"; supplierId = 8; categoryId = 3; unitPrice = 9; unitsInStock = 25; discontinued = false});
  products.add(20, {name = "Sir Rodney's Marmalade"; supplierId = 8; categoryId = 3; unitPrice = 81; unitsInStock = 40; discontinued = false});

  // ── Orders ─────────────────────────────────────────────────────

  let orders : Map.Map<Nat, {
    customerId : Text;
    employeeId : Nat;
    orderDate : Text;
    shipCity : Text;
    shipCountry : Text;
  }> = Map.empty();

  orders.add(10248, {customerId = "ALFKI"; employeeId = 5; orderDate = "1996-07-04"; shipCity = "Berlin"; shipCountry = "Germany"});
  orders.add(10249, {customerId = "ANATR"; employeeId = 6; orderDate = "1996-07-05"; shipCity = "Mexico D.F."; shipCountry = "Mexico"});
  orders.add(10250, {customerId = "CACTU"; employeeId = 4; orderDate = "1996-07-08"; shipCity = "Buenos Aires"; shipCountry = "Argentina"});
  orders.add(10251, {customerId = "ALFKI"; employeeId = 3; orderDate = "1996-07-08"; shipCity = "Berlin"; shipCountry = "Germany"});
  orders.add(10252, {customerId = "BLONP"; employeeId = 4; orderDate = "1996-07-09"; shipCity = "Strasbourg"; shipCountry = "France"});
  orders.add(10253, {customerId = "CACTU"; employeeId = 3; orderDate = "1996-07-10"; shipCity = "Buenos Aires"; shipCountry = "Argentina"});
  orders.add(10254, {customerId = "CHOPS"; employeeId = 5; orderDate = "1996-07-11"; shipCity = "Bern"; shipCountry = "Switzerland"});
  orders.add(10255, {customerId = "BLAUS"; employeeId = 9; orderDate = "1996-07-12"; shipCity = "Mannheim"; shipCountry = "Germany"});
  orders.add(10256, {customerId = "AROUT"; employeeId = 3; orderDate = "1996-07-15"; shipCity = "London"; shipCountry = "UK"});
  orders.add(10257, {customerId = "BERGS"; employeeId = 4; orderDate = "1996-07-16"; shipCity = "Lulea"; shipCountry = "Sweden"});
  orders.add(10258, {customerId = "CENTC"; employeeId = 1; orderDate = "1996-07-17"; shipCity = "Mexico D.F."; shipCountry = "Mexico"});
  orders.add(10259, {customerId = "CENTC"; employeeId = 4; orderDate = "1996-07-18"; shipCity = "Mexico D.F."; shipCountry = "Mexico"});
  orders.add(10260, {customerId = "BOTTM"; employeeId = 4; orderDate = "1996-07-19"; shipCity = "Tsawassen"; shipCountry = "Canada"});
  orders.add(10261, {customerId = "CACTU"; employeeId = 4; orderDate = "1996-07-19"; shipCity = "Buenos Aires"; shipCountry = "Argentina"});
  orders.add(10262, {customerId = "BONAP"; employeeId = 8; orderDate = "1996-07-22"; shipCity = "Marseille"; shipCountry = "France"});
  orders.add(10263, {customerId = "BOTTM"; employeeId = 9; orderDate = "1996-07-23"; shipCity = "Tsawassen"; shipCountry = "Canada"});
  orders.add(10264, {customerId = "AROUT"; employeeId = 6; orderDate = "1996-07-24"; shipCity = "London"; shipCountry = "UK"});
  orders.add(10265, {customerId = "BLONP"; employeeId = 2; orderDate = "1996-07-25"; shipCity = "Strasbourg"; shipCountry = "France"});
  orders.add(10266, {customerId = "COMMI"; employeeId = 3; orderDate = "1996-07-26"; shipCity = "Sao Paulo"; shipCountry = "Brazil"});
  orders.add(10267, {customerId = "ALFKI"; employeeId = 4; orderDate = "1996-07-29"; shipCity = "Berlin"; shipCountry = "Germany"});

  // ── Order Details ──────────────────────────────────────────────

  let orderDetails : Map.Map<Nat, {
    orderId : Nat;
    productId : Nat;
    unitPrice : Nat;
    quantity : Nat;
  }> = Map.empty();

  orderDetails.add(1,  {orderId = 10248; productId = 11; unitPrice = 14; quantity = 12});
  orderDetails.add(2,  {orderId = 10248; productId = 19; unitPrice = 9; quantity = 10});
  orderDetails.add(3,  {orderId = 10248; productId = 1; unitPrice = 18; quantity = 5});
  orderDetails.add(4,  {orderId = 10249; productId = 14; unitPrice = 23; quantity = 9});
  orderDetails.add(5,  {orderId = 10249; productId = 2; unitPrice = 19; quantity = 40});
  orderDetails.add(6,  {orderId = 10250; productId = 3; unitPrice = 10; quantity = 10});
  orderDetails.add(7,  {orderId = 10250; productId = 6; unitPrice = 25; quantity = 5});
  orderDetails.add(8,  {orderId = 10250; productId = 16; unitPrice = 17; quantity = 35});
  orderDetails.add(9,  {orderId = 10251; productId = 1; unitPrice = 18; quantity = 15});
  orderDetails.add(10, {orderId = 10251; productId = 13; unitPrice = 6; quantity = 6});
  orderDetails.add(11, {orderId = 10252; productId = 20; unitPrice = 81; quantity = 40});
  orderDetails.add(12, {orderId = 10252; productId = 12; unitPrice = 38; quantity = 25});
  orderDetails.add(13, {orderId = 10253; productId = 10; unitPrice = 31; quantity = 20});
  orderDetails.add(14, {orderId = 10253; productId = 18; unitPrice = 62; quantity = 42});
  orderDetails.add(15, {orderId = 10254; productId = 1; unitPrice = 18; quantity = 15});
  orderDetails.add(16, {orderId = 10254; productId = 8; unitPrice = 40; quantity = 21});
  orderDetails.add(17, {orderId = 10255; productId = 4; unitPrice = 22; quantity = 20});
  orderDetails.add(18, {orderId = 10255; productId = 16; unitPrice = 17; quantity = 35});
  orderDetails.add(19, {orderId = 10256; productId = 7; unitPrice = 30; quantity = 12});
  orderDetails.add(20, {orderId = 10256; productId = 15; unitPrice = 16; quantity = 25});
  orderDetails.add(21, {orderId = 10257; productId = 11; unitPrice = 14; quantity = 6});
  orderDetails.add(22, {orderId = 10257; productId = 20; unitPrice = 81; quantity = 15});
  orderDetails.add(23, {orderId = 10258; productId = 2; unitPrice = 19; quantity = 50});
  orderDetails.add(24, {orderId = 10258; productId = 5; unitPrice = 21; quantity = 65});
  orderDetails.add(25, {orderId = 10259; productId = 14; unitPrice = 23; quantity = 12});
  orderDetails.add(26, {orderId = 10260; productId = 1; unitPrice = 18; quantity = 10});
  orderDetails.add(27, {orderId = 10260; productId = 11; unitPrice = 14; quantity = 4});
  orderDetails.add(28, {orderId = 10261; productId = 19; unitPrice = 9; quantity = 6});
  orderDetails.add(29, {orderId = 10261; productId = 3; unitPrice = 10; quantity = 20});
  orderDetails.add(30, {orderId = 10262; productId = 12; unitPrice = 38; quantity = 12});
  orderDetails.add(31, {orderId = 10262; productId = 16; unitPrice = 17; quantity = 15});
  orderDetails.add(32, {orderId = 10263; productId = 10; unitPrice = 31; quantity = 10});
  orderDetails.add(33, {orderId = 10263; productId = 18; unitPrice = 62; quantity = 24});
  orderDetails.add(34, {orderId = 10264; productId = 4; unitPrice = 22; quantity = 30});
  orderDetails.add(35, {orderId = 10265; productId = 6; unitPrice = 25; quantity = 10});
  orderDetails.add(36, {orderId = 10265; productId = 8; unitPrice = 40; quantity = 10});
  orderDetails.add(37, {orderId = 10266; productId = 14; unitPrice = 23; quantity = 8});
  orderDetails.add(38, {orderId = 10266; productId = 2; unitPrice = 19; quantity = 12});
  orderDetails.add(39, {orderId = 10267; productId = 1; unitPrice = 18; quantity = 25});
  orderDetails.add(40, {orderId = 10267; productId = 7; unitPrice = 30; quantity = 15});

  // ── Unicode Character Table ────────────────────────────────────

  func natToHex(n : Nat) : Text {
    let hexChars : [Text] = [
      "0","1","2","3","4","5","6","7",
      "8","9","A","B","C","D","E","F"
    ];
    if (n == 0) return "0000";
    var result = "";
    var val = n;
    while (val > 0) {
      result := hexChars[val % 16] # result;
      val := val / 16;
    };
    while (result.size() < 4) {
      result := "0" # result;
    };
    result;
  };

  let unicode : Map.Map<Nat, {
    decimal : Nat;
    hex : Text;
    char : Text;
  }> = Map.empty();


  // U+0000 to U+D7FF (before surrogate range)
  for (i in Nat.range(0, 0xD7FF)) {
    unicode.add(i, {
      decimal = i;
      hex = natToHex(i);
      char = Char.toText(Char.fromNat32(Nat32.fromNat(i)));
    });
  };

/* omit for test
  // U+E000 to U+10FFFF (after surrogate range)
  for (i in Nat.range(0xE000, 0x10FFFF)) {
    unicode.add(i, {
      decimal = i;
      hex = natToHex(i);
      char = Char.toText(Char.fromNat32(Nat32.fromNat(i)));
    });
  };
*/
  // ── Summary (non-paginated) ────────────────────────────────────

  let summary = {
    totalCategories = 8 : Nat;
    totalSuppliers = 10 : Nat;
    totalProducts = 20 : Nat;
    totalCustomers = 15 : Nat;
    totalEmployees = 9 : Nat;
    totalOrders = 20 : Nat;
    totalOrderDetails = 40 : Nat;
    totalUnicode = 1_112_064 : Nat;
  };

  ignore summary;

  public func go() : async () {
    let views = actor (debug_show (Principal.fromActor(Self))) :
      actor {
        __customers: query (ko: ?Text, count: ?Nat) ->
	  async [(Text,
	         { city: Text;
                   companyName: Text;
                   contactName: Text;
                   contactTitle: Text;
                   country: Text;})]
     };
     Debug.print(debug_show (await views.__customers(null, null))); // show all customers from first key
     Debug.print(debug_show (await views.__customers(?"BOTTM", ?4))); // show at most 4 customers from key ""
   }

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"



