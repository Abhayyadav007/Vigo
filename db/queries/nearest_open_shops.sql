-- "Which shops can serve me right now, soonest first?"
-- The core customer query. Ported to Rust as GET /shops/nearby.
-- :lng / :lat are the customer's coordinates.

SELECT s.name,
       ROUND(ST_Distance(s.location, cust.geo)::numeric) AS metres,
       s.avg_prep_seconds AS prep_s,
       s.acceptance_rate,
       -- prep + travel at ~18 km/h (5 m/s), the number the customer sees
       ROUND((s.avg_prep_seconds + ST_Distance(s.location, cust.geo) / 5.0)
             / 60.0) AS eta_min,
       count(si.id) FILTER (WHERE si.is_available AND si.stock_qty > 0) AS items
FROM shops s
CROSS JOIN (SELECT ST_MakePoint(77.6270, 12.9340)::geography AS geo) cust
LEFT JOIN shop_items si ON si.shop_id = s.id
WHERE s.status = 'active'
  AND s.is_open
  AND s.is_accepting_orders
  AND ST_DWithin(s.location, cust.geo, s.delivery_radius_m)
GROUP BY s.id, cust.geo
ORDER BY eta_min, metres;
