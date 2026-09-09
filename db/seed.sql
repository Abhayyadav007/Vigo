-- vigo dev seed. Idempotent-ish: truncate then insert.
-- Shops are real Bangalore locations so distance queries are meaningful.

TRUNCATE shop_items, shops, shop_owners, products, categories, customers RESTART IDENTITY CASCADE;

INSERT INTO categories (name, sort_order) VALUES
    ('Fruits & Vegetables', 1),
    ('Dairy & Eggs',        2),
    ('Staples',             3),
    ('Beverages',           4),
    ('Snacks',              5),
    ('Personal Care',       6);

INSERT INTO products (category_id, name, brand, unit)
SELECT c.id, v.name, v.brand, v.unit
FROM (VALUES
    ('Dairy & Eggs',        'Taaza Toned Milk',      'Amul',        '500 ml'),
    ('Dairy & Eggs',        'Gold Full Cream Milk',  'Amul',        '500 ml'),
    ('Dairy & Eggs',        'Curd',                  'Nandini',     '400 g'),
    ('Dairy & Eggs',        'Butter',                'Amul',        '100 g'),
    ('Dairy & Eggs',        'Eggs',                  '',            '6 pcs'),
    ('Staples',             'Brown Bread',           'Britannia',   '400 g'),
    ('Staples',             'Whole Wheat Atta',      'Aashirvaad',  '5 kg'),
    ('Staples',             'Basmati Rice',          'India Gate',  '1 kg'),
    ('Staples',             'Iodised Salt',          'Tata',        '1 kg'),
    ('Staples',             'Sunflower Oil',         'Fortune',     '1 L'),
    ('Beverages',           'Tea Premium',           'Tata',        '250 g'),
    ('Beverages',           'Classic Coffee',        'Nescafe',     '50 g'),
    ('Snacks',              '2-Minute Noodles',      'Maggi',       '280 g'),
    ('Snacks',              'Parle-G Biscuits',      'Parle',       '250 g'),
    ('Fruits & Vegetables', 'Onion',                 '',            '1 kg'),
    ('Fruits & Vegetables', 'Tomato',                '',            '1 kg'),
    ('Fruits & Vegetables', 'Potato',                '',            '1 kg'),
    ('Fruits & Vegetables', 'Banana Robusta',        '',            '6 pcs'),
    ('Personal Care',       'Handwash',              'Dettol',      '200 ml'),
    ('Personal Care',       'Strong Teeth Toothpaste','Colgate',    '100 g')
) AS v(cat, name, brand, unit)
JOIN categories c ON c.name = v.cat;

INSERT INTO shop_owners (id, phone, name) VALUES
    ('11111111-1111-1111-1111-111111111111', '+919876543210', 'Ramesh Kumar'),
    ('22222222-2222-2222-2222-222222222222', '+919876543211', 'Suresh Nair'),
    ('33333333-3333-3333-3333-333333333333', '+919876543212', 'Anand Rao');

-- ST_MakePoint(longitude, latitude) - lng FIRST.
INSERT INTO shops (id, owner_id, name, address, location, status, is_open,
                   is_accepting_orders, delivery_radius_m, avg_prep_seconds,
                   acceptance_rate) VALUES
    -- ~300 m from the test point: fast, reliable
    ('aaaaaaaa-0000-0000-0000-000000000001', '11111111-1111-1111-1111-111111111111',
     'Sri Lakshmi Stores', '80 Feet Road, Koramangala 4th Block',
     ST_MakePoint(77.6245, 12.9352)::geography,
     'active', TRUE, TRUE, 2500, 240, 0.960),

    -- ~550 m: slower prep, worse acceptance - should rank below shop 1
    ('aaaaaaaa-0000-0000-0000-000000000002', '22222222-2222-2222-2222-222222222222',
     'Green Basket Supermarket', 'Koramangala 5th Block',
     ST_MakePoint(77.6300, 12.9380)::geography,
     'active', TRUE, TRUE, 2500, 420, 0.880),

    -- open but swamped: must be excluded by is_accepting_orders
    ('aaaaaaaa-0000-0000-0000-000000000003', '33333333-3333-3333-3333-333333333333',
     'Anand Provision Store', 'HSR Layout Sector 2',
     ST_MakePoint(77.6389, 12.9116)::geography,
     'active', TRUE, FALSE, 2500, 600, 0.720),

    -- shutters down: must be excluded by is_open
    ('aaaaaaaa-0000-0000-0000-000000000004', '11111111-1111-1111-1111-111111111111',
     'City Fresh Mart', 'CMH Road, Indiranagar',
     ST_MakePoint(77.6408, 12.9784)::geography,
     'active', FALSE, TRUE, 2500, 300, 0.910),

    -- awaiting approval: must be excluded by status
    ('aaaaaaaa-0000-0000-0000-000000000005', '22222222-2222-2222-2222-222222222222',
     'New Star Traders', '11th Main, Jayanagar 4th Block',
     ST_MakePoint(77.5938, 12.9250)::geography,
     'pending', TRUE, TRUE, 2500, 300, 1.000);

-- Every active shop stocks the full catalogue, at slightly different prices.
INSERT INTO shop_items (shop_id, product_id, price_paise, stock_qty)
SELECT s.id,
       p.id,
       pr.paise + (('x' || substr(md5(s.id::text), 1, 4))::bit(16)::int % 300),
       CASE WHEN p.name IN ('Tomato', 'Onion') THEN 8 ELSE 40 END
FROM shops s
CROSS JOIN products p
JOIN (VALUES
    ('Taaza Toned Milk', 2700), ('Gold Full Cream Milk', 3300), ('Curd', 2500),
    ('Butter', 6200), ('Eggs', 4800), ('Brown Bread', 4500),
    ('Whole Wheat Atta', 27500), ('Basmati Rice', 13500), ('Iodised Salt', 2800),
    ('Sunflower Oil', 15500), ('Tea Premium', 14000), ('Classic Coffee', 19000),
    ('2-Minute Noodles', 5600), ('Parle-G Biscuits', 3000), ('Onion', 4000),
    ('Tomato', 3500), ('Potato', 3000), ('Banana Robusta', 4500),
    ('Handwash', 9900), ('Strong Teeth Toothpaste', 5500)
) AS pr(name, paise) ON pr.name = p.name
WHERE s.status = 'active';

INSERT INTO customers (phone, name) VALUES
    ('+919000000001', 'Test Customer');
