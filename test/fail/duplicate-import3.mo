import { f } "duplicate-import/A";
import { Inner = { f } } "duplicate-import/B";

ignore f;
