-- Back-office roles for launch. ADMIN stays the Super Admin.
--   STORE_MANAGER   one store (users.store_id): inventory, orders, open/closed
--   CATALOG_MANAGER global: categories, products, base pricing
--   SUPPORT_AGENT   global: views orders, cancels them
-- Fleet and city managers come later.
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'STORE_MANAGER';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'CATALOG_MANAGER';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'SUPPORT_AGENT';
