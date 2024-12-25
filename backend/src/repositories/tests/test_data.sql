TRUNCATE TABLE Application, ApplicationTargetableRole, Role, RoleMember, Member;
INSERT INTO Member (user_id, email, first_name, last_name, home_municipality, has_accepted_policies) VALUES
('3e1ab0ea-c56a-457f-961f-13938954bb2b', 'marjakarhu@example.org', 'Taneli', 'Mäkinen', 'Espoo', true),
('11880220-c535-4afe-87a2-c59e213f87f4', 'tiinarautio@example.org', 'Johanna', 'Kettunen', 'Espoo', true),
('ffbf88de-758d-498c-b6b4-6968162f2086', 'marjaana89@example.net', 'Juha', 'Lindfors', 'Espoo', true),
('9842528d-e376-42ed-9416-af820bb397b7', 'anderssonsaara@example.org', 'Markku', 'Andersson', 'Espoo', true),
('e27ea8f8-76a3-4386-a09e-694c5581915e', 'johannahytonen@example.org', 'Riitta', 'Tolonen', 'Espoo', true),
('c13689ec-e7ff-4045-b18e-c501d54262c0', 'lappalainenjohannes@example.com', 'Marjatta', 'Toivonen', 'Espoo', true),
('281114a8-98f6-43ca-9cb7-7cf563dc4b96', 'olivia81@example.net', 'Ilmari', 'Suhonen', 'Espoo', true),
('9707582e-c149-45a7-bae1-4b0f4de4b06f', 'joelsuominen@example.net', 'Antti', 'Nyberg', 'Espoo', true),
('b20e5370-f0ae-4c8f-bdf0-5743b8f85c7a', 'karoliinaylonen@example.net', 'Karoliina', 'Karppinen', 'Espoo', true),
('473d18ff-6fdc-468f-9fcf-66533da375da', 'eemilmakinen@example.org', 'Santeri', 'Kurki', 'Espoo', true);

INSERT INTO Role (name, color, description) VALUES
('prodeko-external-member', '#ffffff', 'Prodeko external member'),
('prodeko-full-member', '#fffff0', 'Prodeko external member'),
('prodeko-alumni', '#ffff0f', 'Prodeko external member'),
('pora-member', '#fff0ff', 'Prodeko external member'),
('root-users', '#ff0fff', 'Prodeko external member'),
('prodeko-board', '#f0ffff', 'Prodeko external member'),
('prodeko-official', '#ff0fff', 'Prodeko external member'),
('prodeko-webbitiimi', '#0fffff', 'Prodeko external member');

INSERT INTO RoleMember (user_id, role_name, valid_from, valid_until) VALUES
('e27ea8f8-76a3-4386-a09e-694c5581915e', 'prodeko-external-member', '2021-01-01', '2022-01-01'),
('9707582e-c149-45a7-bae1-4b0f4de4b06f', 'prodeko-external-member', '2022-01-01', '2023-01-01'),
('9707582e-c149-45a7-bae1-4b0f4de4b06f', 'prodeko-external-member', '2023-01-01', '2023-01-01'),
('9707582e-c149-45a7-bae1-4b0f4de4b06f', 'root-users', '2023-01-01', '2024-01-01'),
('9842528d-e376-42ed-9416-af820bb397b7', 'prodeko-official', '2022-01-01', '2023-01-01'),
('3e1ab0ea-c56a-457f-961f-13938954bb2b', 'prodeko-external-member', '2022-01-01', '2022-02-01'),
('3e1ab0ea-c56a-457f-961f-13938954bb2b', 'prodeko-board', '2022-01-01', '2023-01-01'),
('11880220-c535-4afe-87a2-c59e213f87f4', 'prodeko-board', '2021-01-01', '2022-01-01'),
('e27ea8f8-76a3-4386-a09e-694c5581915e', 'prodeko-webbitiimi', '2023-01-01', '2024-01-01'),
('b20e5370-f0ae-4c8f-bdf0-5743b8f85c7a', 'root-users', '2023-01-01', '2024-01-01'),
('9842528d-e376-42ed-9416-af820bb397b7', 'pora-member', '2024-01-01', '2025-01-01'),
('b20e5370-f0ae-4c8f-bdf0-5743b8f85c7a', 'prodeko-full-member', '2024-01-01', '2025-01-01');

INSERT INTO ApplicationTargetableRole (role_name, valid_until, active, payment_link, optional_roles) VALUES
('prodeko-external-member', '2025-01-01', true, '', '{"pora-member"}'),
('prodeko-full-member', '2025-01-01', true, '', '{"pora-member"}'),
('prodeko-alumni', '2025-01-01', true, '', '{"pora-member"}'),
('pora-member', '2025-01-01', true, '', '{"pora-member"}');
