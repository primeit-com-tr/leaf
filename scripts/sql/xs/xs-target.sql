-- === Schema ===
CREATE USER schema1 IDENTIFIED BY schema1_password_tgt;
GRANT CONNECT, RESOURCE TO schema1;

CREATE USER schema2 IDENTIFIED BY schema2_password_tgt;
GRANT CONNECT, RESOURCE TO schema2;

-- === Tables ===
CREATE TABLE schema1.emp (id NUMBER PRIMARY KEY, name VARCHAR2(50));
CREATE TABLE schema1.dept (dept_id NUMBER PRIMARY KEY);
CREATE TABLE schema2.salary (salary_id NUMBER PRIMARY KEY, amount NUMBER);
CREATE TABLE schema2.bonus (bonus_id NUMBER PRIMARY KEY);

-- === Views ===
CREATE OR REPLACE VIEW schema1.v_emp AS
SELECT id, name FROM schema1.emp;

CREATE OR REPLACE VIEW schema2.v_bonus AS
SELECT bonus_id FROM schema2.bonus;

-- === Types ===
CREATE OR REPLACE TYPE schema1.address_t AS OBJECT (
    street VARCHAR2(100),
    city   VARCHAR2(100),
    zip    VARCHAR2(10)
);
/

CREATE OR REPLACE TYPE schema2.department_t AS OBJECT (
    dept_name VARCHAR2(50),
    manager   VARCHAR2(50)
);
/

-- === Procedures ===
CREATE OR REPLACE PROCEDURE schema1.add_emp(p_id NUMBER, p_name VARCHAR2) AS
BEGIN
    INSERT INTO schema1.emp(id, name) VALUES (p_id, p_name);
END;
/

CREATE OR REPLACE PROCEDURE schema2.add_bonus(p_bonus_id NUMBER) AS
BEGIN
    INSERT INTO schema2.bonus(bonus_id) VALUES (p_bonus_id);
END;
/

-- === Packages ===
CREATE OR REPLACE PACKAGE schema1.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER, p_name VARCHAR2);
END emp_pkg;
/

CREATE OR REPLACE PACKAGE BODY schema1.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER, p_name VARCHAR2) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Emp: ' || p_id || ' - ' || p_name);
    END;
END emp_pkg;
/

CREATE OR REPLACE PACKAGE schema2.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER, p_amount NUMBER);
END salary_pkg;
/

CREATE OR REPLACE PACKAGE BODY schema2.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER, p_amount NUMBER) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Salary ID: ' || p_id || ' Amount: ' || p_amount);
    END;
END salary_pkg;
/

-- Optional cleanup script to drop users, which you may want to run before a full create/test run
-- DROP USER schema1 CASCADE;
-- DROP USER schema2 CASCADE;
