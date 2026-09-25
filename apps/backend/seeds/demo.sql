-- Demo data for local development: `cargo run -p backend -- seed-demo`.
-- Idempotent (fixed UUIDs + ON CONFLICT). Barcodes use the GS1 in-store
-- prefix 200-299, so none are real product EANs. Generated; see git history.

BEGIN;

INSERT INTO dark_stores (id, code, name, address, location, service_area) VALUES
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'BLR-IND-01', 'Vigo Indiranagar', '100 Feet Rd, HAL 2nd Stage, Indiranagar, Bengaluru 560038',
   ST_SetSRID(ST_MakePoint(77.6412, 12.9719), 4326)::geography,
   ST_GeomFromGeoJSON('{"type": "Polygon", "coordinates": [[[77.66515, 12.985466], [77.6412, 12.999032], [77.61725, 12.985466], [77.61725, 12.958334], [77.6412, 12.944768], [77.66515, 12.958334], [77.66515, 12.985466]]]}')::geography),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'BLR-KOR-01', 'Vigo Koramangala', '80 Feet Rd, 4th Block, Koramangala, Bengaluru 560034',
   ST_SetSRID(ST_MakePoint(77.6245, 12.9352), 4326)::geography,
   ST_GeomFromGeoJSON('{"type": "Polygon", "coordinates": [[[77.648446, 12.948766], [77.6245, 12.962332], [77.600554, 12.948766], [77.600554, 12.921634], [77.6245, 12.908068], [77.648446, 12.921634], [77.648446, 12.948766]]]}')::geography)
ON CONFLICT (id) DO NOTHING;

INSERT INTO categories (id, name, slug, sort_order) VALUES
  ('9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Fruits & Vegetables', 'fruits-vegetables', 10),
  ('122c9077-c380-595b-8a16-ab7a47260716', 'Dairy, Bread & Eggs', 'dairy-bread-eggs', 20),
  ('73212f47-e355-5f11-8620-5347fb384c03', 'Atta, Rice & Dal', 'atta-rice-dal', 30),
  ('32cbb0e2-6b04-5c7f-8028-2d00ebcb5163', 'Snacks & Munchies', 'snacks-munchies', 40),
  ('4a0f2686-3dab-5d55-a1cb-28bd8c982435', 'Cold Drinks & Juices', 'cold-drinks-juices', 50),
  ('3248e3cb-a0c3-5d7b-bb87-468540d0b0e1', 'Personal Care', 'personal-care', 60)
ON CONFLICT (id) DO NOTHING;

INSERT INTO products (id, category_id, name, slug, brand, unit_label, barcode, mrp_paise, price_paise) VALUES
  ('fe3d57dc-3730-5c5f-ba73-5a29e03ffc8b', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Onion', 'fresho-onion-1-kg', 'Fresho', '1 kg', '2000000000001', 6000, 4500),
  ('e78a910e-dddc-54cd-8a61-f44cebadf0ce', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Tomato - Hybrid', 'fresho-tomato-hybrid-500-g', 'Fresho', '500 g', '2000000000002', 4000, 2900),
  ('4aaa92a2-78f1-5b46-915f-8eab174ce487', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Potato', 'fresho-potato-1-kg', 'Fresho', '1 kg', '2000000000003', 5000, 3900),
  ('87a2cec0-90e8-538c-a7a5-d8061ba56408', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Banana - Robusta', 'fresho-banana-robusta-6-pcs', 'Fresho', '6 pcs', '2000000000004', 6000, 4800),
  ('40f3c732-8f09-5d5e-b507-54ff148f55f0', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Coriander Leaves', 'fresho-coriander-leaves-100-g', 'Fresho', '100 g', '2000000000005', 2000, 1200),
  ('3d4207cd-776e-5cba-ac47-ee5058e91f11', '9d3f0505-a786-52dc-848e-9d18348bb2ea', 'Apple - Shimla', 'fresho-apple-shimla-4-pcs', 'Fresho', '4 pcs', '2000000000006', 18000, 14900),
  ('4fc0f5fb-3c20-5c2c-bec7-e936156f77e8', '122c9077-c380-595b-8a16-ab7a47260716', 'Taaza Toned Milk', 'amul-taaza-toned-milk-500-ml', 'Amul', '500 ml', '2000000000007', 2800, 2800),
  ('9bc41812-24f9-5c1a-a0bb-c2e1d5737166', '122c9077-c380-595b-8a16-ab7a47260716', 'Salted Butter', 'amul-salted-butter-100-g', 'Amul', '100 g', '2000000000008', 6200, 6000),
  ('cd093fe2-43fe-5baf-8dc7-cc60c4dc368c', '122c9077-c380-595b-8a16-ab7a47260716', 'Brown Bread', 'modern-brown-bread-400-g', 'Modern', '400 g', '2000000000009', 5500, 5000),
  ('f82ed778-a003-5989-93aa-10294e0ef239', '122c9077-c380-595b-8a16-ab7a47260716', 'Farm Fresh Eggs', 'eggoz-farm-fresh-eggs-6-pcs', 'Eggoz', '6 pcs', '2000000000010', 7200, 6600),
  ('63a7c2fc-e5e5-5ea4-aa0f-3d96758bb0a9', '73212f47-e355-5f11-8620-5347fb384c03', 'Whole Wheat Atta', 'aashirvaad-whole-wheat-atta-5-kg', 'Aashirvaad', '5 kg', '2000000000011', 31000, 26500),
  ('09cbef19-3ee9-51b2-8ac0-c722085ed3b7', '73212f47-e355-5f11-8620-5347fb384c03', 'Basmati Rice - Classic', 'india-gate-basmati-rice-classic-1-kg', 'India Gate', '1 kg', '2000000000012', 19900, 16500),
  ('161d4bee-fbe4-5896-b206-d60741fcf7e3', '73212f47-e355-5f11-8620-5347fb384c03', 'Toor Dal', 'tata-sampann-toor-dal-1-kg', 'Tata Sampann', '1 kg', '2000000000013', 22000, 18900),
  ('884b4d4d-1cb9-5b0a-a85e-89497efedae3', '73212f47-e355-5f11-8620-5347fb384c03', 'Iodised Salt', 'tata-iodised-salt-1-kg', 'Tata', '1 kg', '2000000000014', 3000, 2800),
  ('aa1c46d8-aeed-57d0-915b-9e24e08dd410', '32cbb0e2-6b04-5c7f-8028-2d00ebcb5163', 'Classic Salted Chips', 'lay-s-classic-salted-chips-52-g', 'Lay''s', '52 g', '2000000000015', 2000, 2000),
  ('0ec129cf-ab0b-5bfd-8dee-5f577b9f989c', '32cbb0e2-6b04-5c7f-8028-2d00ebcb5163', 'Aloo Bhujia', 'haldiram-s-aloo-bhujia-400-g', 'Haldiram''s', '400 g', '2000000000016', 11000, 9900),
  ('7ad15532-1679-502f-a083-86fbbaa575ac', '32cbb0e2-6b04-5c7f-8028-2d00ebcb5163', 'Marie Gold Biscuits', 'britannia-marie-gold-biscuits-250-g', 'Britannia', '250 g', '2000000000017', 4000, 3600),
  ('4716f33c-34e4-5e9e-a2cd-d726d327a699', '32cbb0e2-6b04-5c7f-8028-2d00ebcb5163', 'Dairy Milk Chocolate', 'cadbury-dairy-milk-chocolate-50-g', 'Cadbury', '50 g', '2000000000018', 5000, 4800),
  ('9cc83a6f-1fa1-5364-a3b6-bb5115b514cd', '4a0f2686-3dab-5d55-a1cb-28bd8c982435', 'Coca-Cola', 'coca-cola-coca-cola-750-ml', 'Coca-Cola', '750 ml', '2000000000019', 4000, 3800),
  ('13bc9533-4b3b-567c-a661-8254ab1d04c8', '4a0f2686-3dab-5d55-a1cb-28bd8c982435', 'Mixed Fruit Juice', 'real-mixed-fruit-juice-1-l', 'Real', '1 L', '2000000000020', 13000, 11500),
  ('0c7d5784-ce01-5a74-9896-7fb86bb910af', '4a0f2686-3dab-5d55-a1cb-28bd8c982435', 'Packaged Drinking Water', 'bisleri-packaged-drinking-water-1-l', 'Bisleri', '1 L', '2000000000021', 2000, 2000),
  ('77ce7d95-3163-581c-9326-ebea516c6036', '3248e3cb-a0c3-5d7b-bb87-468540d0b0e1', 'Neem Face Wash', 'himalaya-neem-face-wash-150-ml', 'Himalaya', '150 ml', '2000000000022', 21000, 17900),
  ('f7e885ea-1bf6-55d1-86a6-6d784d4df8f1', '3248e3cb-a0c3-5d7b-bb87-468540d0b0e1', 'Toothpaste - Strong Teeth', 'colgate-toothpaste-strong-teeth-200-g', 'Colgate', '200 g', '2000000000023', 13400, 11800),
  ('667f2377-0c9f-532f-849d-1a86dc4ee991', '3248e3cb-a0c3-5d7b-bb87-468540d0b0e1', 'Bathing Soap', 'dove-bathing-soap-3-x-100-g', 'Dove', '3 x 100 g', '2000000000024', 21000, 18900)
ON CONFLICT (id) DO NOTHING;

-- Stock: every product in Indiranagar; Koramangala lacks personal care, has a
-- couple of items sold out and a cheaper local price on milk.
INSERT INTO store_inventory (store_id, product_id, quantity, bin_location, price_override_paise) VALUES
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'fe3d57dc-3730-5c5f-ba73-5a29e03ffc8b', 15, 'A-01-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'e78a910e-dddc-54cd-8a61-f44cebadf0ce', 22, 'A-02-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '4aaa92a2-78f1-5b46-915f-8eab174ce487', 29, 'A-03-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '87a2cec0-90e8-538c-a7a5-d8061ba56408', 36, 'A-04-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '40f3c732-8f09-5d5e-b507-54ff148f55f0', 43, 'A-01-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '3d4207cd-776e-5cba-ac47-ee5058e91f11', 50, 'A-02-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '4fc0f5fb-3c20-5c2c-bec7-e936156f77e8', 57, 'B-03-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '9bc41812-24f9-5c1a-a0bb-c2e1d5737166', 64, 'B-04-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'cd093fe2-43fe-5baf-8dc7-cc60c4dc368c', 71, 'B-01-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'f82ed778-a003-5989-93aa-10294e0ef239', 18, 'B-02-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '63a7c2fc-e5e5-5ea4-aa0f-3d96758bb0a9', 25, 'C-03-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '09cbef19-3ee9-51b2-8ac0-c722085ed3b7', 32, 'C-04-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '161d4bee-fbe4-5896-b206-d60741fcf7e3', 39, 'C-01-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '884b4d4d-1cb9-5b0a-a85e-89497efedae3', 46, 'C-02-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'aa1c46d8-aeed-57d0-915b-9e24e08dd410', 53, 'D-03-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '0ec129cf-ab0b-5bfd-8dee-5f577b9f989c', 60, 'D-04-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '7ad15532-1679-502f-a083-86fbbaa575ac', 67, 'D-01-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '4716f33c-34e4-5e9e-a2cd-d726d327a699', 74, 'D-02-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '9cc83a6f-1fa1-5364-a3b6-bb5115b514cd', 21, 'E-03-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '13bc9533-4b3b-567c-a661-8254ab1d04c8', 28, 'E-04-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '0c7d5784-ce01-5a74-9896-7fb86bb910af', 35, 'E-01-3', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '77ce7d95-3163-581c-9326-ebea516c6036', 42, 'F-02-1', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', 'f7e885ea-1bf6-55d1-86a6-6d784d4df8f1', 49, 'F-03-2', NULL),
  ('91e5ed5b-c475-5393-bb00-be8c4ee02d98', '667f2377-0c9f-532f-849d-1a86dc4ee991', 56, 'F-04-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'fe3d57dc-3730-5c5f-ba73-5a29e03ffc8b', 26, 'A-01-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'e78a910e-dddc-54cd-8a61-f44cebadf0ce', 33, 'A-02-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '4aaa92a2-78f1-5b46-915f-8eab174ce487', 40, 'A-03-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '87a2cec0-90e8-538c-a7a5-d8061ba56408', 0, 'A-04-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '40f3c732-8f09-5d5e-b507-54ff148f55f0', 54, 'A-01-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '3d4207cd-776e-5cba-ac47-ee5058e91f11', 61, 'A-02-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '4fc0f5fb-3c20-5c2c-bec7-e936156f77e8', 68, 'B-03-1', 2600),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '9bc41812-24f9-5c1a-a0bb-c2e1d5737166', 15, 'B-04-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'cd093fe2-43fe-5baf-8dc7-cc60c4dc368c', 22, 'B-01-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'f82ed778-a003-5989-93aa-10294e0ef239', 29, 'B-02-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '63a7c2fc-e5e5-5ea4-aa0f-3d96758bb0a9', 36, 'C-03-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '09cbef19-3ee9-51b2-8ac0-c722085ed3b7', 43, 'C-04-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '161d4bee-fbe4-5896-b206-d60741fcf7e3', 50, 'C-01-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '884b4d4d-1cb9-5b0a-a85e-89497efedae3', 57, 'C-02-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', 'aa1c46d8-aeed-57d0-915b-9e24e08dd410', 64, 'D-03-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '0ec129cf-ab0b-5bfd-8dee-5f577b9f989c', 71, 'D-04-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '7ad15532-1679-502f-a083-86fbbaa575ac', 18, 'D-01-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '4716f33c-34e4-5e9e-a2cd-d726d327a699', 0, 'D-02-3', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '9cc83a6f-1fa1-5364-a3b6-bb5115b514cd', 32, 'E-03-1', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '13bc9533-4b3b-567c-a661-8254ab1d04c8', 39, 'E-04-2', NULL),
  ('1918cb61-9f0a-5557-97b3-53d52045d60f', '0c7d5784-ce01-5a74-9896-7fb86bb910af', 46, 'E-01-3', NULL)
ON CONFLICT (store_id, product_id) DO NOTHING;

COMMIT;
