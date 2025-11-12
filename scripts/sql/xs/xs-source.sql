-- === Schema ===
CREATE USER schema1 IDENTIFIED BY schema1_password;
GRANT CONNECT, RESOURCE TO schema1;

CREATE USER schema2 IDENTIFIED BY schema2_password;
GRANT CONNECT, RESOURCE TO schema2;

-- === Tables ===
CREATE TABLE schema1.emp (id NUMBER PRIMARY KEY);
CREATE TABLE schema1.dept (dept_id NUMBER PRIMARY KEY);
CREATE TABLE schema2.emp (id NUMBER PRIMARY KEY);
CREATE TABLE schema2.salary (salary_id NUMBER PRIMARY KEY);

-- === Views ===
CREATE OR REPLACE VIEW schema1.v_emp AS
SELECT id FROM schema1.emp;

CREATE OR REPLACE VIEW schema2.v_salary AS
SELECT salary_id FROM schema2.salary;

-- === Types ===
CREATE OR REPLACE TYPE schema1.address_t AS OBJECT (
    street VARCHAR2(50),
    city   VARCHAR2(50)
);
/

CREATE OR REPLACE TYPE schema2.department_t AS OBJECT (
    dept_name VARCHAR2(50)
);
/

-- === Procedures ===
CREATE OR REPLACE PROCEDURE schema1.add_emp(p_id NUMBER) AS
BEGIN
    INSERT INTO schema1.emp(id) VALUES (p_id);
END;
/

CREATE OR REPLACE PROCEDURE schema2.add_salary(p_salary_id NUMBER) AS
BEGIN
    INSERT INTO schema2.salary(salary_id) VALUES (p_salary_id);
END;
/

-- === Packages ===
CREATE OR REPLACE PACKAGE schema1.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER);
END emp_pkg;
/

CREATE OR REPLACE PACKAGE BODY schema1.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Emp ID: ' || p_id);
    END;
END emp_pkg;
/

CREATE OR REPLACE PACKAGE schema2.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER);
END salary_pkg;
/

CREATE OR REPLACE PACKAGE BODY schema2.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Salary ID: ' || p_id);
    END;
END salary_pkg;
/

-- Optional cleanup script to drop users, which you may want to run before a full create/test run
-- DROP USER schema1 CASCADE;
-- DROP USER schema2 CASCADE;
